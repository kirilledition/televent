/* eslint-disable @typescript-eslint/no-require-imports */
const { performance } = require('perf_hooks');

const EVENT_COUNT = 10000;
const ITERATIONS = 100;

function generateEvents(count) {
  const events = [];
  const start = new Date(2023, 0, 1).getTime();
  const end = new Date(2025, 11, 31).getTime();

  for (let i = 0; i < count; i++) {
    const d = new Date(start + Math.random() * (end - start));
    const date = d.toISOString().split('T')[0];
    const time = d.toISOString().split('T')[1].substring(0, 5); // HH:MM
    events.push({
      id: String(i),
      date,
      time: Math.random() > 0.5 ? time : undefined, // Some have time, some don't
    });
  }
  return events;
}

const events = generateEvents(EVENT_COUNT);

console.log(`Benchmarking sort with ${EVENT_COUNT} events over ${ITERATIONS} iterations...`);

// Baseline: Date parsing sort
let totalBaselineTime = 0;
for (let i = 0; i < ITERATIONS; i++) {
  const data = [...events];
  const start = performance.now();
  data.sort((a, b) => {
    const dateA = new Date(`${a.date} ${a.time || '00:00'}`);
    const dateB = new Date(`${b.date} ${b.time || '00:00'}`);
    return dateA.getTime() - dateB.getTime();
  });
  totalBaselineTime += performance.now() - start;
}
const avgBaseline = totalBaselineTime / ITERATIONS;
console.log(`Baseline (Date parsing): ${avgBaseline.toFixed(4)} ms`);

// Optimized: String comparison sort
let totalOptimizedTime = 0;
for (let i = 0; i < ITERATIONS; i++) {
  const data = [...events];
  const start = performance.now();
  data.sort((a, b) => {
    // Primary sort by date
    if (a.date < b.date) return -1;
    if (a.date > b.date) return 1;

    // Secondary sort by time
    const timeA = a.time || '00:00';
    const timeB = b.time || '00:00';
    if (timeA < timeB) return -1;
    if (timeA > timeB) return 1;

    return 0;
  });
  totalOptimizedTime += performance.now() - start;
}
const avgOptimized = totalOptimizedTime / ITERATIONS;
console.log(`Optimized (String compare): ${avgOptimized.toFixed(4)} ms`);

const improvement = ((avgBaseline - avgOptimized) / avgBaseline * 100).toFixed(2);
console.log(`Improvement: ${improvement}% faster`);
