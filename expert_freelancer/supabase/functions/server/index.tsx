import { Hono } from "npm:hono";
import { cors } from "npm:hono/cors";
import { logger } from "npm:hono/logger";
import * as kv from "./kv_store.tsx";
const app = new Hono();

// Enable logger
app.use('*', logger(console.log));

// Enable CORS for all routes and methods
app.use(
  "/*",
  cors({
    origin: "*",
    allowHeaders: ["Content-Type", "Authorization"],
    allowMethods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
    exposeHeaders: ["Content-Length"],
    maxAge: 600,
  }),
);

// Health check endpoint
app.get("/make-server-7109387c/health", (c) => {
  return c.json({ status: "ok" });
});

// Get all events
app.get("/make-server-7109387c/events", async (c) => {
  try {
    const events = await kv.getByPrefix("event:");
    return c.json({ events });
  } catch (error) {
    console.error("Error fetching events:", error);
    return c.json({ error: "Failed to fetch events", details: String(error) }, 500);
  }
});

// Create a new event
app.post("/make-server-7109387c/events", async (c) => {
  try {
    const event = await c.req.json();
    const eventId = `event:${Date.now()}`;
    await kv.set(eventId, event);
    return c.json({ id: eventId, ...event });
  } catch (error) {
    console.error("Error creating event:", error);
    return c.json({ error: "Failed to create event", details: String(error) }, 500);
  }
});

// Delete an event
app.delete("/make-server-7109387c/events/:id", async (c) => {
  try {
    const id = c.req.param("id");
    await kv.del(id);
    return c.json({ success: true });
  } catch (error) {
    console.error("Error deleting event:", error);
    return c.json({ error: "Failed to delete event", details: String(error) }, 500);
  }
});

// Update an event
app.put("/make-server-7109387c/events/:id", async (c) => {
  try {
    const id = c.req.param("id");
    const event = await c.req.json();
    await kv.set(id, event);
    return c.json({ id, ...event });
  } catch (error) {
    console.error("Error updating event:", error);
    return c.json({ error: "Failed to update event", details: String(error) }, 500);
  }
});

Deno.serve(app.fetch);