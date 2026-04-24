import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "sans-serif",
        ],
        mono: [
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Monaco",
          "Consolas",
          "Liberation Mono",
          "monospace",
        ],
      },
      colors: {
        lab: {
          canvas: "#111418",
          panel: "#171a20",
          panel2: "#1d2128",
          line: "#313843",
          muted: "#8f9aaa",
          text: "#e7ebf2",
          accent: "#56b6c2",
          warn: "#d8ad5b",
          danger: "#d06464",
          green: "#7fb069",
        },
      },
      boxShadow: {
        focus: "0 0 0 1px #56b6c2, 0 0 0 4px rgba(86, 182, 194, 0.16)",
      },
    },
  },
  plugins: [],
} satisfies Config;
