#!/usr/bin/env node
// Generates tests/oracle/regexp-vectors.json by running every vector through
// Node's own RegExp implementation. Each entry records
// {pattern, flags, input, op, replacement?, expected} where op is one of
// "test" | "replace" | "split" | "search".
//
// Constraints kept in sync with the Rust engine subset
// (crates/tsonic_rust_js/src/regexp/):
// - only constructs from the supported subset appear here;
// - split vectors never use capturing groups (the Rust engine rejects them
//   because JS splices capture values into split output);
// - empty-match advancement vectors stay within the BMP.

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const cases = [];
const test = (pattern, flags, input) => cases.push({ pattern, flags, input, op: "test" });
const search = (pattern, flags, input) => cases.push({ pattern, flags, input, op: "search" });
const split = (pattern, flags, input) => cases.push({ pattern, flags, input, op: "split" });
const replace = (pattern, flags, input, replacement) =>
  cases.push({ pattern, flags, input, op: "replace", replacement });

// --- literals, dot, escapes -------------------------------------------------
test("abc", "", "xxabcxx");
test("abc", "", "xxabxcx");
test("a.c", "", "abc");
test("a.c", "", "a\nc");
test("a\\.c", "", "abc");
test("a\\.c", "", "a.c");
test("\\+\\*\\?", "", "x+*?y");
test("a\\/b", "", "a/b");
test("\\\\", "", "back\\slash");
test("\\n", "", "line1\nline2");
test("\\t\\r\\f\\v", "", "a\t\r\f\vb");
test("\\x41\\u0042", "", "zABz");
test("\\0", "", "a\0b");
search("a.c", "", "zzabc");
search("q", "", "zzabc");

// --- character classes ------------------------------------------------------
test("[abc]", "", "zzz");
test("[abc]", "", "zbz");
test("[a-z]+", "", "HELLO there");
test("[^a-z]", "", "abcZ");
test("[^a-z]", "", "abc");
test("[a-zA-Z0-9_]+", "", "!!id_42!!");
test("[-abc]", "", "x-y");
test("[abc-]", "", "x-y");
test("[\\]]", "", "a]b");
test("[\\d]+", "", "abc123");
test("[^\\d]+", "", "123");
test("[\\w\\s]+", "", "a b");
test("[a-c][x-z]", "", "by");
test("[a-c][x-z]", "", "bw");
test("[]a", "", "a");
test("[^]", "", "\n");
search("[0-9]", "", "abc42");

// --- class escapes in and out of classes ------------------------------------
test("\\d+", "", "order 66");
test("\\D+", "", "123");
test("\\D+", "", "123abc");
test("\\w+", "", "hi_there9");
test("\\W", "", "abc_123");
test("\\W", "", "abc!123");
test("\\s", "", "a b");
test("\\s", "", "a b");
test("\\s", "", "ab");
test("\\S+", "", "   x  ");
search("\\d", "", "abc123");
search("\\S", "", "   !");

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
test(".*c", "", "abcabc");
replace(".*c", "", "abcabcd", "[$&]");
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
test("[^a-z]", "i", "AbC");
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
