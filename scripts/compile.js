import { Deno } from "deno";

async function compileApp() {
  const entryPoint = "./core/pacm.js";
  const output = "./dist/pacm.exe";

  try {
    const result = await Deno.compile(entryPoint, output);
    console.log("Compilation successful:", result);
  } catch (error) {
    console.error("Compilation failed:", error);
  }
}

export { compileApp };
