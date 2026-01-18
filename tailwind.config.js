/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./crates/web/src/**/*.{rs,html}",
    "./public/**/*.html",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Dark mode first design
        zinc: {
          950: '#09090b',
        },
      },
    },
  },
  plugins: [],
}
