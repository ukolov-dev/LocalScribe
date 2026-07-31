import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import { build } from "esbuild";

const outfile = "/private/tmp/localscribe-feedback-test.mjs";

await build({
  bundle: true,
  entryPoints: ["src/lib/feedback.ts"],
  format: "esm",
  outfile,
  platform: "node"
});

const feedback = await import(pathToFileURL(outfile).href);

assert.deepEqual(feedback.copySuccessFeedback("Copied"), {
  error: "",
  notice: "Copied"
});
assert.deepEqual(feedback.copyFailureFeedback("Failed"), {
  error: "Failed",
  notice: ""
});
