/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './index.html',
    './src/**/*.{js,ts,jsx,tsx}',
    '../../packages/ui/src/**/*.{js,ts,jsx,tsx}',
  ],
  darkMode: ['class'],
  theme: {
    fontFamily: {
      sans: ['Inter', '"Segoe UI"', '"PingFang SC"', 'sans-serif'],
      mono: ['"Roboto Mono"', 'monospace'],
    },
    extend: {
      colors: {
        presentation: 'rgb(var(--presentation) / <alpha-value>)',
        'text-primary': 'rgb(var(--text-primary) / <alpha-value>)',
        'text-secondary': 'rgb(var(--text-secondary) / <alpha-value>)',
        'text-secondary-alt': 'rgb(var(--text-secondary-alt) / <alpha-value>)',
        'surface-active-alt': 'rgb(var(--surface-active-alt) / <alpha-value>)',
        'surface-hover': 'rgb(var(--surface-hover) / <alpha-value>)',
        'surface-primary': 'rgb(var(--surface-primary) / <alpha-value>)',
        'surface-primary-alt': 'rgb(var(--surface-primary-alt) / <alpha-value>)',
        'surface-secondary': 'rgb(var(--surface-secondary) / <alpha-value>)',
        'surface-secondary-alt': 'rgb(var(--surface-secondary-alt) / <alpha-value>)',
        'surface-chat': 'rgb(var(--surface-chat) / <alpha-value>)',
        'border-light': 'rgb(var(--border-light) / <alpha-value>)',
        'border-heavy': 'rgb(var(--border-heavy) / <alpha-value>)',
      },
      boxShadow: {
        stroke: '0 0 0 1px rgb(var(--border-light) / 1)',
      },
      animation: {
        'fade-in': 'fadeIn 0.25s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: 0 },
          '100%': { opacity: 1 },
        },
      },
    },
  },
  plugins: [],
}
