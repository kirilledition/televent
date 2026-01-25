const http = require('http');

// Auth Header
const AUTH_HEADER = "tma auth_date=1700000000&query_id=AAGyswdAAAAAAALLB0A&user=%7B%22id%22%3A123456789%2C%22first_name%22%3A%22Test%22%2C%22last_name%22%3A%22User%22%2C%22username%22%3A%22testuser%22%2C%22language_code%22%3A%22en%22%2C%22is_premium%22%3Afalse%2C%22allows_write_to_pm%22%3Atrue%7D&hash=075e0d126e8e57060d9fdca6599f95482a4fdb97521e1a937f7c5dd8f6190719";

function postEvent(event) {
    const data = JSON.stringify(event);
    const options = {
        hostname: 'localhost',
        port: 3001,
        path: '/api/events',
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': AUTH_HEADER,
            'Content-Length': data.length
        }
    };

    const req = http.request(options, (res) => {
        console.log(`Status: ${res.statusCode} for ${event.summary}`);
        if (res.statusCode >= 400) {
            res.on('data', d => process.stdout.write(d));
        }
    });

    req.on('error', (error) => {
        console.error(`Error: ${error.message}`);
    });

    req.write(data);
    req.end();
}

const now = new Date();

function addHours(date, h) {
    const d = new Date(date);
    d.setTime(d.getTime() + (h * 60 * 60 * 1000));
    return d.toISOString();
}

function addDays(date, days) {
    const d = new Date(date);
    d.setDate(d.getDate() + days);
    return d;
}

// Event 1: Team Sync (Tomorrow)
const tomorrow = addDays(now, 1);
tomorrow.setHours(10, 0, 0, 0);

postEvent({
    uid: crypto.randomUUID(),
    summary: "Team Sync",
    description: "Weekly sync with the engineering team.",
    location: "Google Meet",
    start: tomorrow.toISOString(),
    end: addHours(tomorrow, 1),
    is_all_day: false,
    timezone: "UTC"
});

// Event 2: Lunch (Tomorrow)
tomorrow.setHours(13, 0, 0, 0);
postEvent({
    uid: crypto.randomUUID(),
    summary: "Lunch with Sarah",
    description: "Discussing new project",
    location: "Italian Place",
    start: tomorrow.toISOString(),
    end: addHours(tomorrow, 1.5),
    is_all_day: false,
    timezone: "UTC"
});

// Event 3: Code Review (Day after)
const dayAfter = addDays(now, 2);
dayAfter.setHours(15, 0, 0, 0);
postEvent({
    uid: crypto.randomUUID(),
    summary: "Code Review",
    description: "Reviewing PR #42",
    location: "Office",
    start: dayAfter.toISOString(),
    end: addHours(dayAfter, 1),
    is_all_day: false,
    timezone: "UTC"
});

// Event 4: Hackathon (Next Fri)
const nextFri = addDays(now, 5); // Rough approx
nextFri.setHours(9, 0, 0, 0);
const nextFriEnd = new Date(nextFri);
nextFriEnd.setHours(18, 0, 0, 0);

postEvent({
    uid: crypto.randomUUID(),
    summary: "Internal Hackathon",
    description: "Building cool stuff!",
    location: "HQ",
    start: nextFri.toISOString(),
    end: nextFriEnd.toISOString(),
    is_all_day: true,
    timezone: "UTC"
});
