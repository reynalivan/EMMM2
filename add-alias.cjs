const fs = require('fs');

let tsconfig = fs.readFileSync('tsconfig.json', 'utf8');
if (!tsconfig.includes('baseUrl')) {
    tsconfig = tsconfig.replace('"compilerOptions": {', '"compilerOptions": {\n    "baseUrl": ".",\n    "paths": { "@/*": ["src/*"] },');
    fs.writeFileSync('tsconfig.json', tsconfig, 'utf8');
}

let viteConfig = fs.readFileSync('vite.config.ts', 'utf8');
if (!viteConfig.includes('resolve: {')) {
    const resolveBlock = '  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },';
    viteConfig = viteConfig.replace('plugins: [react(), tailwindcss()],', 'plugins: [react(), tailwindcss()],\n' + resolveBlock);
    fs.writeFileSync('vite.config.ts', viteConfig, 'utf8');
}
console.log('Alias configured');
