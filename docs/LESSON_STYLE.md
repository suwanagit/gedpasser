# Lesson style

Every lesson in this app follows these rules. They are written for adults
who never learned this math, or who haven't done math in 10 years or more.
The short cards in `content/cards/` are not lessons: they are the **Quick
review**, a cheat sheet for someone who has already done the lesson.

## The rules

1. **One idea per lesson.** If a title needs a comma or "and", split it.
   "Factors, multiples, distributing" becomes at least three lessons: the
   greatest common factor, the least common multiple, and the distributive
   property.
2. **Assume the reader knows only basic add, subtract, multiply and
   divide.** Any other math word (factor, multiple, divisible, product,
   variable) is defined in plain words the first time it appears, with a
   number example right next to it.
3. **Order every lesson like this:**
   1. **Why it matters**: one or two sentences, a real situation where this
      comes up (money, cooking, splitting things evenly).
   2. **The idea in plain words**, no symbols yet.
   3. **A worked example** with small, easy numbers, one step per line. Each
      step says WHY, not just what.
   4. **A second worked example**, slightly harder.
   5. **Only now: the general rule or formula**, described as shorthand for
      what they just did. Say what each letter stands for.
   6. **A common mistake**, shown and corrected.
   7. **Try it**: 2 or 3 questions inside the lesson, with instant feedback
      and a full worked solution when they get it wrong.
   8. **Quick recap**: the short cheat-sheet version goes here.
4. **Never state an answer without showing how to get it.** "The GCF of 18
   and 30 is 6" is not allowed. Listing the factors of both numbers and
   picking the biggest shared one is.
5. **Never bring in a new number without saying where it came from.** If 38
   becomes 40 − 2, say: "38 is close to 40, which is easier to multiply, so
   think of 38 as 40 − 2."
6. **Examples must match the rule exactly as written.** If the rule shows a
   plus, the first example uses plus. Show the minus version separately and
   say it works the same way.
7. **Reading level: about 6th grade.** Short sentences. Talk to the reader
   as "you". Never write "simply", "obviously" or "just".
8. **One step on screen at a time.** Lessons are paged, not one long card.

## Where lessons live

One file per skill, `content/lessons/<skill>.toml`, with one `[[lesson]]`
per idea. A principle (one official GED code, such as `Q.1.b`) can have
several lessons. The learner sees the parts in the order of the rules above;
`recap` sits near the top of the entry only because TOML needs it there:

```toml
[[lesson]]
id = "q1b-gcf"                    # q + principle code without dots + a short name
principle = "Q.1.b"
status = "draft"                  # only `cargo content-approve` sets reviewed
title = "The greatest common factor"   # no commas; avoid "and"
why = '''
One or two sentences: a real situation where this comes up.
'''
idea = '''
The idea in plain words. Define every new word, with a number example
right next to it. No symbols yet.
'''
recap = [                          # 2 to 4 short lines, shown last
  "...",                           # (TOML needs plain fields before the [[...]] tables)
]

[[lesson.example]]                # exactly two; the second a little harder
title = "12 and 18"
steps = [
  { do = "List the factors of 12: 1, 2, 3, 4, 6, 12", why = "These are all the numbers that divide 12 with nothing left over." },
  { do = "...", why = "..." },
]
answer = "The GCF of 12 and 18 is 6."

[[lesson.example]]
title = "..."
steps = [ ... ]
answer = "..."

[lesson.rule]
text = '''
The general rule, as shorthand for what the examples did.
'''
terms = [                          # optional: each letter or word in the rule
  { term = "GCF", means = "greatest common factor: the biggest number that divides both" },
]

[lesson.mistake]
wrong = "The mistaken work, shown."
right = "The same problem done correctly."
why = "Why the mistake happens and how to avoid it."

[[lesson.try]]                     # 2 or 3
prompt = "..."
choices = ["...", "..."]           # leave out for a typed answer
answer = "4"
check = "gcd(8, 12)"               # required when the answer is a number
solution = '''
The full worked solution, shown when the answer is wrong.
'''

```

Text conventions are the same as everywhere else: math goes in `$...$`, a
literal dollar sign is `\$`.

## What checks what

`cargo content` enforces:

- the structure: exactly two examples, every step has a `do` and a `why`,
  2 or 3 try-its, each with a solution, and 2 to 4 recap lines;
- every numeric try-it answer matches its `check` expression;
- no "simply", "obviously" or "just", and no comma in the title (errors);
- "and" in the title, sentences over 20 words, and a reading level above
  about grade 7 (warnings). Reading level is measured on the words only,
  with math left out.

The independent reviewer checks the rest: every new term is defined with a
number example (rule 2), every answer is shown being worked out (rule 4),
every new number is explained (rule 5), and the examples match the rule as
written (rule 6). See `content/reviews/CHECKLIST.md`.
