import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Deployed to Vercel at the domain root, so no base prefix. Serving this from a
// subpath instead would need `base` set to match, or every asset 404s.
export default defineConfig({
  plugins: [react(), tailwindcss()],
});
