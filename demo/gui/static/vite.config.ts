import { defineConfig, type Plugin } from 'vite';
import { createReadStream, existsSync, statSync } from 'fs';
import { resolve, join, extname } from 'path';
import vue from '@vitejs/plugin-vue';

// Serve demo data files directly during development.
// This allows /data/config/* and /data/input/* to resolve without the Rust backend.
function serveDataFiles(): Plugin {
  const dataDir = resolve(__dirname, '../../data');
  const mimeTypes: Record<string, string> = {
    '.json': 'application/json',
    '.toml': 'text/plain',
    '.csv': 'text/csv',
    '.md': 'text/markdown',
  };
  return {
    name: 'serve-data-files',
    configureServer(server) {
      server.middlewares.use('/data', (req, res, next) => {
        const filePath = join(dataDir, decodeURIComponent(req.url || ''));
        if (existsSync(filePath) && statSync(filePath).isFile()) {
          const ext = extname(filePath);
          res.setHeader('Content-Type', mimeTypes[ext] || 'application/octet-stream');
          res.setHeader('Access-Control-Allow-Origin', '*');
          createReadStream(filePath).pipe(res);
        } else {
          next();
        }
      });
    },
  };
}

export default defineConfig({
  root: '.',
  base: '/',
  plugins: [vue(), serveDataFiles()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    sourcemap: true,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
      },
      output: {
        manualChunks: {
          vendor: ['vue', 'vue-router', 'pinia', 'chart.js'],
        },
      },
    },
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
});
