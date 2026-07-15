/** @type {import('tailwindcss').Config} */

module.exports = {
  content: {
    files: ["index.html", "./src/**/*.rs"],
  },
  theme: {
    extend: {
      colors: {
        "bg-base": "var(--bg-base)",
        "bg-card": "var(--bg-card)",
        "bg-surface": "var(--bg-surface)",
        "bg-modal": "var(--bg-modal)",
        "text-primary": "var(--text-primary)",
        "text-secondary": "var(--text-secondary)",
        "text-tertiary": "var(--text-tertiary)",
        accent: "var(--accent)",
        "accent-strong": "var(--accent-strong)",
        "accent-weak": "var(--accent-weak)",
        success: "var(--success)",
        error: "var(--error)",
        warning: "var(--warning)",
      },
      borderColor: {
        DEFAULT: "var(--border)",
        strong: "var(--border-strong)",
      },
      borderRadius: {
        xl: "var(--radius-xl)",
        "2xl": "var(--radius-2xl)",
        full: "var(--radius-full)",
      },
      transitionTimingFunction: {
        spring: "var(--ease-spring)",
      },
      backgroundImage: {
        accent: "var(--accent-gradient)",
      },
    },
  },
  plugins: [],
};
