#!/usr/bin/env node
// Generates tests/oracle/regexp-vectors.json by running every vector through
// Node's own RegExp implementation. Each entry records
// {pattern, flags, input, op, replacement?, calls?, setLastIndex?, expected}
// where op is one of "test" | "test-sequence" | "replace" | "split" |
// "search" | "exec" | "match" | "matchAll" | "set-lastindex".
//
// Expected shapes for the match-carrier ops:
// - exec: `calls` sequential re.exec() calls on one regexp instance; each
//   step is {match: null | {text, index, groups}, lastIndex} so lastIndex
//   progression (and its reset to 0 on a null result) is asserted.
// - test-sequence: `calls` sequential re.test() calls on one regexp
//   instance; each step is {result, lastIndex}, asserting that a global
//   test advances lastIndex to the match end and resets it to 0 on failure
//   (and that a non-global test leaves it untouched).
// - match: null, or an array of matched texts (g flag), or a single
//   {text, index, groups} object (no g flag).
// - set-lastindex: `setLastIndex` is written to re.lastIndex, then one
//   re.exec() runs; expected is {result, lastIndex} where result is the
//   match text or null. These vectors prove that a writable lastIndex is
//   exact for non-nullable patterns even when it lands between the two
//   code units of a surrogate pair: no atom in the accepted subset can
//   match a lone surrogate, so a mid-pair start is equivalent to the next
//   char boundary. Nullable patterns must not appear here — they can match
//   empty AT a mid-pair position (Node: /a*/g with lastIndex 1 on "💚"
//   matches "" at index 1), so the Rust engine rejects the lastIndex write
//   itself for nullable patterns.
// - matchAll: an array of {text, index, groups}, or {throws: "TypeError"}
//   when the regexp lacks the g flag.
// Group values are null for unmatched optional groups; `index`/`lastIndex`
// are UTF-16 code-unit offsets.
//
// Constraints kept in sync with the Rust engine subset
// (crates/tsonic_rust_js/src/regexp/):
// - only constructs from the supported subset appear here;
// - no `.`, negated classes (`[^...]`, `\D \W \S`), astral chars inside
//   classes, or class ranges reaching past U+D7FF: their non-`u` semantics
//   are defined over UTF-16 code units (Node's /./.exec("😀") yields a lone
//   high surrogate), so the Rust engine rejects them at construction;
// - split vectors never use capturing groups (the Rust engine rejects them
//   because JS splices capture values into split output);
// - empty-match advancement vectors stay within the BMP: the Rust engine
//   fails closed (Unsupported) when a nullable pattern iterates over astral
//   input, because JS advances one UTF-16 code unit past an empty match,
//   which can split a surrogate pair. Astral inputs appear here only with
//   non-nullable patterns, where behavior is exact.

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const cases = [];
const test = (pattern, flags, input) => cases.push({ pattern, flags, input, op: "test" });
const search = (pattern, flags, input) => cases.push({ pattern, flags, input, op: "search" });
const split = (pattern, flags, input) => cases.push({ pattern, flags, input, op: "split" });
const replace = (pattern, flags, input, replacement) =>
  cases.push({ pattern, flags, input, op: "replace", replacement });
const exec = (pattern, flags, input, calls) =>
  cases.push({ pattern, flags, input, op: "exec", calls });
const testSequence = (pattern, flags, input, calls) =>
  cases.push({ pattern, flags, input, op: "test-sequence", calls });
const match = (pattern, flags, input) => cases.push({ pattern, flags, input, op: "match" });
const setLastIndex = (pattern, flags, input, value) =>
  cases.push({ pattern, flags, input, op: "set-lastindex", setLastIndex: value });
const matchAll = (pattern, flags, input) => cases.push({ pattern, flags, input, op: "matchAll" });

// --- literals and escapes -------------------------------------------------
test("abc", "", "xxabcxx");
test("abc", "", "xxabxcx");
test("a\\.c", "", "abc");
test("a\\.c", "", "a.c");
test("\\+\\*\\?", "", "x+*?y");
test("a\\/b", "", "a/b");
test("\\\\", "", "back\\slash");
test("\\n", "", "line1\nline2");
test("\\t\\r\\f\\v", "", "a\t\r\f\vb");
test("\\x41\\u0042", "", "zABz");
test("\\0", "", "a\0b");
search("q", "", "zzabc");

// --- character classes ------------------------------------------------------
test("[abc]", "", "zzz");
test("[abc]", "", "zbz");
test("[a-z]+", "", "HELLO there");
test("[a-zA-Z0-9_]+", "", "!!id_42!!");
test("[-abc]", "", "x-y");
test("[abc-]", "", "x-y");
test("[\\]]", "", "a]b");
test("[\\d]+", "", "abc123");
test("[\\w\\s]+", "", "a b");
test("[a-c][x-z]", "", "by");
test("[a-c][x-z]", "", "bw");
test("[]a", "", "a");
search("[0-9]", "", "abc42");
// Ranges capped at U+D7FF (the retained boundary) stay exact, including
// over astral input: neither half of a surrogate pair (Node) nor an
// astral scalar (Rust) falls inside a range that stops at U+D7FF.
test("[\\u0100-\\ud7ff]", "", "aЖb");
test("[\\x61-\\ud7ff]+", "", "😀");

// --- class escapes in and out of classes ------------------------------------
test("\\d+", "", "order 66");
test("\\w+", "", "hi_there9");
test("\\s", "", "a b");
test("\\s", "", "a b");
test("\\s", "", "ab");
search("\\d", "", "abc123");

// --- quantifiers ------------------------------------------------------------
test("ab*c", "", "ac");
test("ab*c", "", "abbbc");
test("ab+c", "", "ac");
test("ab+c", "", "abbc");
test("ab?c", "", "ac");
test("ab?c", "", "abc");
test("ab?c", "", "abbc");
test("a{3}", "", "aa");
test("a{3}", "", "aaa");
test("a{2,}", "", "aa");
test("a{2,}", "", "a");
test("a{2,4}b", "", "aaaaab");
test("a{2,4}b", "", "ab");
test("a{0,2}b", "", "b");
search("ba{2,3}", "", "xbaab");
replace("a{2,4}", "", "aaaaaa", "<$&>");
replace("x*", "", "yyy", "-");

// --- greedy backtracking ----------------------------------------------------
test("[a-z]*c", "", "abcabc");
replace("[a-z]*c", "", "abcabcd", "[$&]");
replace("a+a", "", "aaaa", "<$&>");
replace("[a-z]*bc", "", "xxabcbc!", "<$&>");

// --- anchors with and without m ---------------------------------------------
test("^abc", "", "abcdef");
test("^abc", "", "xabc");
test("abc$", "", "xxabc");
test("abc$", "", "abcx");
test("^abc$", "", "abc");
test("^b", "", "a\nb");
test("^b", "m", "a\nb");
test("b$", "", "b\na");
test("b$", "m", "b\na");
test("^$", "", "");
test("^$", "m", "a\n\nb");
replace("^a", "g", "aaa", "X");
replace("^", "m", "a\nb", "> ");
replace("^", "gm", "a\nb", "> ");
replace("$", "gm", "a\nb", "!");
search("^b$", "m", "aa\nb\ncc");

// --- alternation precedence -------------------------------------------------
test("a|b", "", "zzb");
test("ab|cd", "", "acd");
replace("a|ab", "", "abc", "<$&>");
replace("ab|a", "", "abc", "<$&>");
replace("^x|y$", "g", "xzy", "-");
test("cat|dog|bird", "", "hotdog!");
replace("a(b|c)d", "", "zacdz", "[$1]");
search("b|c", "", "abc");

// --- groups and $n substitution ----------------------------------------------
replace("(\\d+)-(\\d+)", "", "call 12-34 now", "$2-$1");
replace("(\\w+) (\\w+)", "", "hello world", "$2 $1");
replace("(a+)(b*)", "", "aab", "[$1|$2]");
replace("(a+)(b*)", "", "aa", "[$1|$2]");
replace("(a)|(b)", "g", "ab", "<$1:$2>");
replace("((a)(b))", "", "ab", "$1-$2-$3");
replace("(a)", "", "a", "$2");
replace("(a)", "", "a", "$0");
replace("(a)", "", "a", "$$1");
replace("(a)", "", "a", "$&$&");
replace("(a)b", "", "xaby", "$`");
replace("(a)b", "", "xaby", "$'");
replace("(\\d)(\\d)(\\d)(\\d)(\\d)(\\d)(\\d)(\\d)(\\d)(\\d)(\\d)", "", "01234567890", "$11|$10|$1");
replace("a(?:bc)d", "", "xabcdy", "[$&]");
test("(?:ab)+", "", "ababab");

// --- i flag -------------------------------------------------------------------
test("abc", "i", "xAbCy");
test("[a-z]+", "i", "HELLO");
test("ÉCOLE", "i", "école");
test("école", "i", "ÉCOLE");
test("[à-ö]", "i", "Ä");
replace("hello", "i", "say Hello twice Hello", "bye");
replace("hello", "gi", "say Hello twice heLLo", "bye");
search("b", "i", "aBc");

// --- g vs non-g replace -------------------------------------------------------
replace("o", "", "foo boo", "0");
replace("o", "g", "foo boo", "0");
replace("\\d+", "g", "a1b22c333", "#");
replace("\\d+", "", "a1b22c333", "#");
replace("\\s+", "g", "a  b\tc", " ");
replace("q", "g", "aaa", "z");

// --- split ---------------------------------------------------------------------
split(",", "", "a,b,c");
split(",", "", "a,,c");
split(",", "", ",a,");
split(",", "", "");
split("", "", "abc");
split("", "", "");
split("\\s*,\\s*", "", "a , b,c ,d");
split("\\s+", "", "a b  c");
split("(?:,|;)", "", "a,b;c");
split("x", "", "abc");
split("o", "g", "foo boo");
split("\\d", "", "a1b2c");
split("a*", "", "baaab");

// --- empty-match edge cases -----------------------------------------------------
replace("", "", "abc", "-");
replace("", "g", "abc", "-");
replace("a*", "g", "bab", "-");
replace("b*", "g", "abc", "!");
replace("$", "", "abc", "!");
replace("^", "", "abc", ">");
test("a*", "", "");
search("a*", "", "bbb");

// --- unicode BMP chars -----------------------------------------------------------
test("你好", "", "说你好吗");
replace("好", "g", "好上加好", "x");
split("、", "", "一、二、三");
test("[α-ω]+", "", "χαος");
search("好", "", "说你好");
replace("[é]", "g", "café résumé", "e");

// --- exec: sequential g calls with lastIndex progression ---------------------
exec("\\d+", "g", "a1b22c333", 5);
exec("a", "g", "aaa", 5);
exec("a*", "g", "aab", 4);
exec("x", "g", "abc", 2);
exec("(\\w)(\\d)?", "g", "a1 b", 4);
exec("o", "", "foo", 3);
exec("b", "g", "abcabc", 4);
exec("^a", "g", "aaa", 3);
exec("\\d", "g", "你1好2", 4);
exec("(a+)(b+)", "g", "aabbab", 4);
exec("$", "g", "ab", 3);
exec("a|b", "g", "xaybz", 4);

// --- test: sequential g calls with lastIndex progression ----------------------
testSequence("\\d+", "g", "a1b22c333", 5);
testSequence("a", "g", "aaa", 5);
testSequence("x", "g", "abc", 2);
testSequence("b", "g", "abcabc", 4);
testSequence("^a", "g", "aaa", 3);
testSequence("(\\w)(\\d)?", "g", "a1 b", 4);
testSequence("\\d", "g", "你1好2", 4);
testSequence("a*", "g", "aab", 4);
testSequence("o", "", "foo", 3);
testSequence("q", "", "abc", 2);

// --- empty-match iteration over BMP input (exact vs Node) ---------------------
replace("x?", "g", "abc", "-");
replace("(?:a|)", "g", "zaz", "<$&>");
split("x?", "", "abc");
split("a{0,2}", "", "bab");
match("", "g", "ab");
match("b*", "g", "abc");
matchAll("x?", "g", "ab");
matchAll("b{0,2}", "g", "abcb");

// --- non-nullable patterns over astral input (exact vs Node) ------------------
// Astral literals stay unquantified: without the `u` flag Node reads 💚 as
// two code units, so a quantifier would bind to the trailing surrogate only
// (the Rust engine rejects a directly quantified astral literal for that
// reason). A grouped astral literal repeats the whole pair and is exact.
test("(?:💚)+", "", "a💚💚b");
replace("(?:💚)+", "g", "a💚💚b💚", "x");
test("💚", "", "a💚b");
search("💚", "", "a💚b");
replace("💚", "g", "a💚b💚c", "x");
replace("💚", "g", "a💚💚b", "[$&]");
split("💚", "", "a💚b💚c");
match("\\d+", "g", "💚1💚22");
match("💚", "", "a💚b");
matchAll("💚|\\d", "g", "a💚💚b1");
exec("💚", "g", "a💚b💚", 3);
// Nullable pattern over astral input: natural-flow exec stays on char
// boundaries (the empty match at 0 leaves lastIndex at 0 on every call), so
// it is exact even though manual lastIndex writes are rejected for /a*/.
exec("a*", "g", "💚a", 3);

// --- writable lastIndex landing inside/around surrogate pairs ------------------
// Non-nullable patterns only: the Rust engine rejects manual lastIndex
// writes on nullable patterns (see the set-lastindex note above).
// "ab😀cd😀x": a=0 b=1 😀=2..3 c=4 d=5 😀=6..7 x=8 (length 9).
setLastIndex("[a-z]+", "g", "ab😀cd😀x", 2); // at a high surrogate (pair start)
setLastIndex("[a-z]+", "g", "ab😀cd😀x", 3); // at the low surrogate (mid-pair)
setLastIndex("[a-z]+", "g", "ab😀cd😀x", 4); // just after a pair
setLastIndex("[a-z]+", "g", "ab😀cd😀x", 7); // mid-pair of the second astral char
setLastIndex("[a-z]+", "g", "ab😀cd😀x", 9); // at end of input
setLastIndex("[a-z]+", "g", "ab😀cd😀x", 42); // beyond input
// "1😀2😀3": 1=0 😀=1..2 2=3 😀=4..5 3=6 (length 7).
setLastIndex("\\d", "g", "1😀2😀3", 1); // at a high surrogate
setLastIndex("\\d", "g", "1😀2😀3", 2); // mid-pair
setLastIndex("\\d", "g", "1😀2😀3", 3); // just after a pair
setLastIndex("\\d", "g", "1😀2😀3", 5); // mid-pair of the second astral char
setLastIndex("\\d", "g", "1😀2😀3", 8); // beyond input
// "💚x💚": 💚=0..1 x=2 💚=3..4 (length 5).
setLastIndex("x", "g", "💚x💚", 0); // at a pair start
setLastIndex("x", "g", "💚x💚", 1); // mid-pair, match begins at the next boundary
setLastIndex("x", "g", "💚x💚", 3); // at the second pair start (no match left)
setLastIndex("x", "g", "💚x💚", 4); // mid-pair with no match left

// --- match with the g flag ----------------------------------------------------
match("\\d+", "g", "a1b22c333");
match("q", "g", "abc");
match("a*", "g", "baab");
match("[aeiou]", "g", "education");
match("(\\d)(\\d)", "g", "1234 56");
match("好", "g", "好上加好");
match("^", "gm", "a\nb");
match("\\w+", "g", "  ");
match("ab?", "g", "abaab");

// --- match without the g flag ---------------------------------------------------
match("\\d+", "", "a1b22");
match("(\\w+) (\\w+)", "", "hello world x");
match("q", "", "abc");
match("(a)|(b)", "", "zb");
match("好", "", "说你好");
match("a{2,3}", "i", "xAAAy");
match("(?:ab)+", "", "zababz");
match("^$", "m", "a\n\nb");

// --- matchAll -------------------------------------------------------------------
matchAll("\\d+", "g", "a1b22c333");
matchAll("(\\w)(\\d)", "g", "a1 b2 c");
matchAll("q", "g", "abc");
matchAll("a*", "g", "baab");
matchAll("(a)|(b)", "g", "ab");
matchAll("\\d", "g", "你1好2");
matchAll("\\d+", "", "a1");
matchAll("(a+)(b*)", "g", "aabab");
matchAll("^[a-z]", "gm", "ab\ncd");

const matchRecord = (m) => ({
  text: m[0],
  index: m.index,
  groups: m.slice(1).map((group) => (group === undefined ? null : group)),
});

const results = cases.map((entry) => {
  const re = new RegExp(entry.pattern, entry.flags);
  let expected;
  switch (entry.op) {
    case "test":
      expected = re.test(entry.input);
      break;
    case "search":
      expected = entry.input.search(re);
      break;
    case "split":
      expected = entry.input.split(re);
      break;
    case "replace":
      expected = entry.input.replace(re, entry.replacement);
      break;
    case "test-sequence": {
      const steps = [];
      for (let call = 0; call < entry.calls; call += 1) {
        steps.push({ result: re.test(entry.input), lastIndex: re.lastIndex });
      }
      expected = steps;
      break;
    }
    case "exec": {
      const steps = [];
      for (let call = 0; call < entry.calls; call += 1) {
        const m = re.exec(entry.input);
        steps.push({
          match: m === null ? null : matchRecord(m),
          lastIndex: re.lastIndex,
        });
      }
      expected = steps;
      break;
    }
    case "match": {
      const m = entry.input.match(re);
      expected = m === null ? null : re.global ? [...m] : matchRecord(m);
      break;
    }
    case "set-lastindex": {
      re.lastIndex = entry.setLastIndex;
      const m = re.exec(entry.input);
      expected = { result: m === null ? null : m[0], lastIndex: re.lastIndex };
      break;
    }
    case "matchAll": {
      try {
        expected = [...entry.input.matchAll(re)].map(matchRecord);
      } catch (error) {
        expected = { throws: error.constructor.name };
      }
      break;
    }
    default:
      throw new Error(`unknown op ${entry.op}`);
  }
  return { ...entry, expected };
});

const here = dirname(fileURLToPath(import.meta.url));
const outPath = join(here, "..", "tests", "oracle", "regexp-vectors.json");
mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify(results, null, 2) + "\n");
console.log(`wrote ${results.length} vectors to ${outPath}`);
