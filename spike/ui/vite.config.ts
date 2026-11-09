import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  base: './',
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  resolve: {
    alias: {
      // Monaco's package exports only map .js files; the trimmed entry also imports CSS.
      'monaco-vs': fileURLToPath(new URL('./node_modules/monaco-editor/esm/vs', import.meta.url)),
    },
  },
  build: { target: 'safari16', chunkSizeWarningLimit: 5000, reportCompressedSize: true },
});
