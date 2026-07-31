import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import { build } from "esbuild";

const outfile = "/private/tmp/localscribe-transcript-filter-test.mjs";

await build({
  bundle: true,
  entryPoints: ["src/lib/transcriptFilter.ts"],
  format: "esm",
  outfile,
  platform: "node"
});

const transcriptFilter = await import(pathToFileURL(outfile).href);

function segment(index, text) {
  return {
    index,
    startMs: index * 1000,
    endMs: (index + 1) * 1000,
    text
  };
}

assert.deepEqual(
  transcriptFilter.filterLiveTranscriptSegments([
    segment(0, "Good morning."),
    segment(1, "Субтитры создавал DimaTorzok"),
    segment(2, "Музыка"),
    segment(3, "without a doubt")
  ]).map((item) => item.text),
  ["Good morning.", "without a doubt"]
);

const acknowledgementRun = [segment(0, "What have you seen during holidays?")];
for (let index = 1; index < 24; index += 1) {
  acknowledgementRun.push(segment(index, index % 2 === 0 ? "Угу." : "Mm-hmm."));
}
acknowledgementRun.push(segment(24, "I have been to Italy."));

const filtered = transcriptFilter.filterLiveTranscriptSegments(acknowledgementRun);
const acknowledgementCount = filtered.filter((item) => {
  const normalized = item.text
    .normalize("NFKC")
    .toLocaleLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim();
  return normalized === "угу" || normalized === "mm hmm";
}).length;

assert.equal(acknowledgementCount, 4);
assert(filtered.some((item) => item.text === "I have been to Italy."));
