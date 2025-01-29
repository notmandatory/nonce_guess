const { fontFamily } = require("tailwindcss/defaultTheme");
const colors = require('tailwindcss/colors')

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["templates/*.html"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter var", ...fontFamily.sans],
      },
      colors: {
        ...colors,
      },
    },
  },
  plugins: [require("@tailwindcss/forms")],
};
