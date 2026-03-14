import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import pkg from "oxc-transform";
const { transform } = pkg;

const source = readFileSync("ts/scanner.ts", "utf8");

const result = transform("scanner.ts", source, {
  typescript: {
    onlyRemoveTypeImports: true,
  },
});

if (result.errors.length > 0) {
  console.error("Transform errors:");
  for (const err of result.errors) {
    console.error(err);
  }
  process.exit(1);
}

mkdirSync("ts/dist", { recursive: true });
writeFileSync("ts/dist/scanner.js", result.code);
console.log("Built ts/dist/scanner.js");
