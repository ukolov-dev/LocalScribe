import type { TranscriptSegment } from "../types/localscribe";

const MAX_CONSECUTIVE_REPEATED_SEGMENTS = 2;
const MAX_REPEATED_LONG_SEGMENTS = 2;
const MAX_REPEATED_SHORT_SEGMENTS = 8;
const MAX_REPEATED_ACKNOWLEDGEMENTS = 4;
const ALPHANUMERIC = /[\p{L}\p{N}]/u;

export function filterLiveTranscriptSegments(segments: TranscriptSegment[]): TranscriptSegment[] {
  const filtered: TranscriptSegment[] = [];
  const seenCounts = new Map<string, number>();
  let lastKey = "";
  let consecutiveCount = 0;

  for (const segment of [...segments].sort((left, right) => left.index - right.index)) {
    const text = segment.text.trim();
    const key = normalizeRepetitionKey(text);
    if (!key) {
      continue;
    }

    if (isKnownCaptionHallucination(key)) {
      continue;
    }

    if (key === lastKey) {
      consecutiveCount += 1;
    } else {
      lastKey = key;
      consecutiveCount = 1;
    }

    const wordCount = key.split(/\s+/).filter(Boolean).length;
    const seenKey = repetitionSeenKey(key);
    const seenCount = seenCounts.get(seenKey) ?? 0;
    const maxSeen = isAcknowledgementKey(key)
      ? MAX_REPEATED_ACKNOWLEDGEMENTS
      : wordCount <= 3
        ? MAX_REPEATED_SHORT_SEGMENTS
        : MAX_REPEATED_LONG_SEGMENTS;

    if (
      consecutiveCount > MAX_CONSECUTIVE_REPEATED_SEGMENTS ||
      seenCount >= maxSeen ||
      hasRunawayRepetition(key)
    ) {
      continue;
    }

    filtered.push({ ...segment, text });
    seenCounts.set(seenKey, seenCount + 1);
  }

  return filtered;
}

export function formatTranscriptSegments(segments: TranscriptSegment[]): string {
  return segments
    .map((segment) => {
      const text = segment.text.trim();
      if (!text) {
        return "";
      }

      const speaker = segment.speaker?.trim();
      return speaker ? `${speaker}: ${text}` : text;
    })
    .filter(Boolean)
    .join("\n");
}

function normalizeRepetitionKey(text: string): string {
  let normalized = "";
  let previousWasSpace = true;

  for (const character of Array.from(text.normalize("NFKC").toLocaleLowerCase())) {
    if (ALPHANUMERIC.test(character)) {
      normalized += character;
      previousWasSpace = false;
    } else if (!previousWasSpace) {
      normalized += " ";
      previousWasSpace = true;
    }
  }

  return normalized.trim();
}

function isKnownCaptionHallucination(key: string): boolean {
  return (
    key === "музыка" ||
    key === "music" ||
    key.startsWith("субтитры создавал") ||
    key.startsWith("субтитры создал") ||
    key.startsWith("субтитры сделал") ||
    key.startsWith("субтитры подготовил") ||
    key.startsWith("subtitles by") ||
    key.startsWith("subtitle by") ||
    key.startsWith("subtitles created by") ||
    key.startsWith("subtitle created by")
  );
}

function repetitionSeenKey(key: string): string {
  return isAcknowledgementKey(key) ? "__acknowledgement__" : key;
}

function isAcknowledgementKey(key: string): boolean {
  return [
    "угу",
    "ага",
    "mm hmm",
    "mhm",
    "mmhmm",
    "uh huh",
    "uhhuh"
  ].includes(key);
}

function hasRunawayRepetition(key: string): boolean {
  const words = key.split(/\s+/).filter(Boolean);
  if (words.length < 6) {
    return false;
  }

  const maxNgram = Math.min(4, Math.floor(words.length / 2));
  for (let ngramSize = 1; ngramSize <= maxNgram; ngramSize += 1) {
    const requiredRepeats = ngramSize === 1 ? 4 : 3;
    let index = 0;

    while (index + ngramSize * requiredRepeats <= words.length) {
      const phrase = words.slice(index, index + ngramSize).join("\u0000");
      let repeats = 1;

      while (index + (repeats + 1) * ngramSize <= words.length) {
        const nextPhrase = words
          .slice(index + repeats * ngramSize, index + (repeats + 1) * ngramSize)
          .join("\u0000");

        if (nextPhrase !== phrase) {
          break;
        }

        repeats += 1;
      }

      if (repeats >= requiredRepeats) {
        return true;
      }

      index += Math.max(repeats, 1) * ngramSize;
    }
  }

  return false;
}
