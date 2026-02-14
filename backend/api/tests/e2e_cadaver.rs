use api::{AppState, create_router};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use moka::future::Cache;
use sqlx::PgPool;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::process::Command;
// use uuid::Uuid; (already imported or unused)

async fn setup_user_and_auth(pool: &PgPool) -> (i64, String, String) {
    let telegram_id: i64 = rand::random::<i64>().abs();
    let username = format!("e2e_user_{}", telegram_id);

    // Insert user
    sqlx::query(
        r#"
        INSERT INTO users (
            telegram_id, telegram_username, timezone,
            sync_token, ctag,
            created_at, updated_at
        )
        VALUES ($1, $2, 'UTC', '0', '0', NOW(), NOW())
        "#,
    )
    .bind(telegram_id)
    .bind(&username)
    .execute(pool)
    .await
    .expect("Failed to create user");

    let password = "test_password_e2e";
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    sqlx::query(
        r#"
        INSERT INTO device_passwords (
            id, user_id, password_hash, device_name, created_at
        ) 
        VALUES (gen_random_uuid(), $1, $2, 'e2e_device', NOW())
        "#,
    )
    .bind(telegram_id) // user_id is telegram_id (BIGINT)
    .bind(password_hash)
    .execute(pool)
    .await
    .expect("Failed to create device password");

    (telegram_id, username, password.to_string())
}

async fn run_cadaver_commands(
    url: &str,
    user: i64,
    pass: &str,
    commands: &[&str],
) -> (bool, String, String) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let netrc_path = temp_dir.path().join(".netrc");

    // Parse host from URL
    let parsed_url = url::Url::parse(url).expect("Failed to parse URL");
    let host = parsed_url.host_str().expect("Failed to get host");

    let netrc_content = format!("machine {} login {} password {}\n", host, user, pass);
    std::fs::write(&netrc_path, netrc_content).expect("Failed to write .netrc");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&netrc_path, std::fs::Permissions::from_mode(0o600))
            .expect("Failed to set netrc permissions");
    }

    let mut child = Command::new("cadaver")
        .arg("-t") // non-interactive
        .arg(url)
        .env("HOME", temp_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start cadaver");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");

    for cmd in commands {
        tracing::debug!("Sending command to cadaver: {}", cmd);
        stdin
            .write_all(format!("{}\n", cmd).as_bytes())
            .await
            .expect("Failed to write command");
    }
    stdin
        .write_all(b"quit\n")
        .await
        .expect("Failed to write quit");
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .expect("Failed to read cadaver output");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("api=trace,info,e2e_cadaver=trace")
        .try_init();
}

#[sqlx::test(migrations = "../migrations")]
async fn test_cadaver_full_flow(pool: PgPool) {
    init_tracing();
    let (telegram_id, _username, password) = setup_user_and_auth(&pool).await;

    let state = AppState {
        pool: pool.clone(),
        auth_cache: Cache::builder()
            .time_to_live(Duration::from_secs(300))
            .build(),
        telegram_bot_token: "test_token".to_string(),
    };

    let app = create_router(state, "*", ".");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().expect("Failed to get addr");

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .expect("Server failed");
    });

    let base_url = format!("http://{}/caldav/{}/", addr, telegram_id);

    // 1. Sanity Check with reqwest
    let client = reqwest::Client::new();
    let res = client
        .request(reqwest::Method::OPTIONS, &base_url)
        .basic_auth(telegram_id.to_string(), Some(password.clone()))
        .send()
        .await
        .expect("Sanity check failed");
    assert_eq!(res.status(), 200);

    // 2. Cadaver: Connection & LS
    let (success, stdout, stderr) = tokio::time::timeout(
        Duration::from_secs(10),
        run_cadaver_commands(&base_url, telegram_id, &password, &["ls"]),
    )
    .await
    .expect("Cadaver connection timed out");

    assert!(
        success,
        "cadaver ls failed!\nSTDOUT: {}\nSTDERR: {}",
        stdout, stderr
    );
    assert!(
        stdout.contains("Listing collection"),
        "Output missing 'Listing collection'"
    );

    // 3. Cadaver: PUT, GET, DELETE
    let _event_uid = "e2e-123";
    let ics_content = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:e2e-123\r\nSUMMARY:E2E Event\r\nDTSTART:20250101T100000Z\r\nDTEND:20250101T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let ics_path = "e2e-123.ics";
    std::fs::write(ics_path, ics_content).expect("Failed to write ics");

    let (success, stdout, stderr) = tokio::time::timeout(
        Duration::from_secs(15),
        run_cadaver_commands(
            &base_url,
            telegram_id,
            &password,
            &[
                &format!("put {}", ics_path),
                "get e2e-123.ics e2e-123-down.ics",
                "delete e2e-123.ics",
            ],
        ),
    )
    .await
    .expect("Cadaver sequence timed out");

    let _ = std::fs::remove_file(ics_path);
    let _ = std::fs::remove_file("e2e-123-down.ics");

    assert!(
        success,
        "cadaver sequence failed!\nSTDOUT: {}\nSTDERR: {}",
        stdout, stderr
    );
    assert!(
        stdout.contains("Uploading"),
        "PUT missing! STDOUT: {}",
        stdout
    );
    assert!(
        stdout.contains("Downloading"),
        "GET missing! STDOUT: {}",
        stdout
    );
    assert!(
        stdout.contains("Deleting"),
        "DELETE missing! STDOUT: {}",
        stdout
    );
}
