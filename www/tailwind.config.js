/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{js,jsx,ts,tsx}"],
  theme: {
    extend: {
			colors: {
				"si-blue": "#5dabf5",
				"si-blue-light": "#7dbcf7",
			}
		},
  },
  plugins: [],
  corePlugins: {
    preflight: false,
  },
  blocklist: ["container"],
};