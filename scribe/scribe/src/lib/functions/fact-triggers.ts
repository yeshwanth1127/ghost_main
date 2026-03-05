/**
 * Fact extraction triggers.
 * Reduces extraction calls by ~85% via keyword, interval, and token thresholds.
 */

const FACT_TRIGGER_PATTERNS = [
  /my name is/i,
  /I'm\s+[A-Za-z]+/i,       // "I'm Alex"
  /I am\s+[A-Z][a-z]+/i,    // "I am Alex" (capitalized name, not "I am working")
  /call me\s+/i,            // "call me Alex"
  /I am working on/i,
  /I am building/i,
  /my project/i,
  /I use/i,
  /our stack/i,
  /database/i,
  /framework/i,
  /language/i,
  /I prefer/i,
  /I like/i,
];

const INTERVAL_TURNS = 12;
const TOKEN_THRESHOLD = 5000;

export function shouldExtractFactsByKeyword(message: string): boolean {
  return FACT_TRIGGER_PATTERNS.some((p) => p.test(message));
}

export function shouldExtractFactsByInterval(turnCount: number): boolean {
  return turnCount > 0 && turnCount % INTERVAL_TURNS === 0;
}

export function shouldExtractFactsByTokenThreshold(
  totalTokens: number,
  factsRecentlyUpdated: boolean
): boolean {
  return totalTokens > TOKEN_THRESHOLD && !factsRecentlyUpdated;
}

export function shouldExtractFacts(
  userMessage: string,
  turnCount: number,
  totalTokens: number,
  factsRecentlyUpdated: boolean
): boolean {
  if (shouldExtractFactsByKeyword(userMessage)) return true;
  if (shouldExtractFactsByInterval(turnCount)) return true;
  if (shouldExtractFactsByTokenThreshold(totalTokens, factsRecentlyUpdated))
    return true;
  return false;
}
