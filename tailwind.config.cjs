/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        brand: {
          50: "#eef0ff",
          100: "#d9ddff",
          200: "#b3b9ff",
          300: "#8d95ff",
          400: "#6771ff",
          500: "#414dff",
          600: "#303acc",
          700: "#242b99",
          800: "#171d66",
          900: "#0b0e33",
        },
        surface: {
          50: "#0f1115",
          100: "#151820",
          200: "#1f2430",
          300: "#2a3142",
          400: "#364055",
        },
      },
      boxShadow: {
        card: "0 12px 35px rgba(15, 17, 21, 0.45)",
        glow: "0 0 35px rgba(65, 77, 255, 0.3)",
      },
    },
  },
  plugins: [require("@tailwindcss/forms")],
};
