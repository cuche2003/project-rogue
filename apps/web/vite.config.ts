import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

export default defineConfig({
  resolve: {
    alias: {
      '#lib': new URL('./src/lib', import.meta.url).pathname
    }
  },
  plugins: [
    sveltekit({
      adapter: adapter(),
      preprocess: vitePreprocess()
    })
  ]
});
