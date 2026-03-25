import { build } from "esbuild";

// Scanner: bundle html5-qrcode into the output
await build({
  entryPoints: ["ts/scanner.ts"],
  outfile: "ts/dist/scanner.js",
  bundle: true,
  format: "iife",
  target: "es2020",
  minify: true,
});
console.log("Built ts/dist/scanner.js");

// Haptics: bundle web-haptics into the output
await build({
  entryPoints: ["ts/haptics.ts"],
  outfile: "ts/dist/haptics.js",
  bundle: true,
  format: "iife",
  target: "es2020",
  minify: true,
});
console.log("Built ts/dist/haptics.js");
