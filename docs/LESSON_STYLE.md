# Lesson style

Every lesson in this app follows these rules. They are written for adults
who never learned this math, or who haven't done math in 10 years or more.

The rules come from Jon's brief, checked against how real educators teach
the same material (see "Where this comes from" at the end). The short cards
in `content/cards/` are not lessons: they are the old **Quick review** for a
whole principle, kept until each principle's lessons exist.

## The rules

1. **One idea per lesson.** If a title needs a comma or "and", split it.
   "Factors, multiples, distributing" becomes at least three lessons: the
   greatest common factor, the least common multiple, and the distributive
   property.
2. **Assume the reader knows only basic add, subtract, multiply and
   divide.** Any other math word (factor, multiple, divisible, product,
   variable) is defined in plain words the first time it appears, with a
   number example right next to it.
3. **Every lesson has these parts, in this order.** Each is one screen.
   1. **Goal.** One line: "You'll learn to …"
   2. **A problem to think about.** A real situation (money, cooking,
      splitting things evenly) posed as a question, before any new word
      appears. Don't answer it yet: example 1 solves it.
   3. **The idea, with a picture.** Each new word in plain language, with a
      picture and a number example right next to it. No formulas yet.
   4. **Example 1 ("watch me").** Small, easy numbers. Shown a step at a
      time. Each step says what we do and why, then shows the written work.
      End with the answer in a full sentence that restates the question,
      then a check. Then answer the opening problem.
   5. **Your turn.** One question like example 1, with new numbers.
   6. **Example 2**, slightly harder, then **your turn** again.
   7. **How to.** The method as numbered steps, each a short command, said to
      be shorthand for what they just did. Say what each letter stands for.
      Add a memory hook.
   8. **Watch out.** One or two common mistakes, shown wrong, then right,
      then why.
   9. **Practice.** Two more questions.
   10. **Quick review**, then **How did that go?** ("I've got it" / "I need
       more practice" / "Show me again").

   Every question shows the full worked solution when the answer is wrong.
4. **Never state an answer without showing how to get it.** "The GCF of 18
   and 30 is 6" is not allowed. Listing the factors of both numbers and
   picking the biggest shared one is.
5. **Never bring in a new number without saying where it came from.** If 38
   becomes 40 − 2, say: "38 is close to 40, which is easier to multiply, so
   think of 38 as 40 − 2."
6. **Examples must match the rule exactly as written.** If the rule shows a
   plus, the first example uses plus. Show the minus version separately and
   say it works the same way.
7. **Reading level: about 6th grade.** Short sentences. Never write
   "simply", "obviously" or "just".
8. **One step on screen at a time.** Lessons are paged, not one long card.

## How to word it

Write the way a good teacher talks a class through a problem.

- **Talk to the reader.** Use "you" for what they do and "we" or "let's"
  for working together: "Let's find the factors of 12."
- **Point things out.** "Notice that 36 ÷ 6 = 6." "Why stop at 4?"
- **Say why before how.** "Divide 12 by 1, 2, 3, and so on. Each time it
  divides evenly, you get a pair of factors."
- **Lay out work the way it's written on paper.** One line per step, with
  labels: "Factors of 12: 1, 2, 3, 4, 6, 12". Mark what matters in
  **bold**.
- **Answer in a full sentence that restates the question.** "The greatest
  common factor of 12 and 18 is 6." Then check it.
- **Name the term after the learner has seen the idea.** The picture of 12
  squares comes before the word "factor" is used on its own.
- **Show a systematic way that tells you when you're done**, not a list
  that appears from nowhere.
- **Respect the reader.** No "easy", no talking down. Adults bring real
  experience: use it.

## Where lessons live

One file per skill, `content/lessons/<skill>.toml`, with one `[[lesson]]`
per idea. A principle (one official GED code, such as `Q.1.b`) can have
several lessons. The sample to copy is `q1b-gcf` in
`content/lessons/q-number-ops.toml`. The fields:

| Field | Holds |
| --- | --- |
| `id`, `principle`, `status`, `title` | `id` is the principle code without dots, then a short name (`q1b-gcf`); `status = "draft"` until reviewed |
| `goal` | One line |
| `how_to`, `hook` | 2–5 short commands, and a memory hook. TOML needs these near the top, before the `[...]` tables, but they are shown after the examples |
| `[lesson.problem]` | `text` (the question) and `answer` (said after example 1) |
| `[lesson.idea]` | `text`, and optional `figures` (for example `{ kind = "array", rows = 2, cols = 6, label = "2 × 6" }`); put a `{{pictures}}` line in the text right where the pictures belong |
| `[[lesson.example]]` (exactly 2) | `title`; `steps`, each `{ say = "what and why", work = ["line", "line"] }`; `answer` (full sentence); `check`; then `[lesson.example.turn]`, the your-turn question |
| `[[lesson.mistake]]` (1 or 2) | `wrong`, `right`, `why` |
| `[[lesson.practice]]` (exactly 2) | A question |
| `[lesson.review]` | `example`: a few lines for the Quick review |

A question has `prompt`, optional `choices` (leave out for a typed answer),
`answer`, `check` (an expression; required when the answer is a number) and
`solution` (the full worked solution).

Text conventions are the same as everywhere else: math goes in `$...$`, a
literal dollar sign is `\$`, and `**bold**` marks what to notice.

## What checks what

`cargo content` enforces:

- the structure: two examples with at least two steps each, a your-turn
  after each, 2–5 how-to steps, one or two mistakes, two practice
  questions, and nothing empty;
- every numeric answer matches its `check` expression;
- no "simply", "obviously" or "just", and no comma in the title (errors);
- "and" in the title, sentences over 20 words, and a reading level above
  about grade 7 (warnings). Reading level is measured on the words only,
  with math left out.

The independent reviewer checks the rest:

- every new term is defined with a number example (rule 2);
- every answer is shown being worked out (rule 4);
- every new number is explained (rule 5);
- the examples match the rule as written (rule 6);
- the wording follows "How to word it".

See `content/reviews/CHECKLIST.md`.

## Where this comes from

Read in November 2026 for this guide. Our lessons are written in our own
words; nothing is copied.

- **Illustrative Mathematics, Grade 6, Unit 7, Lesson 16 "Common Factors"**
  (im.kendallhunt.com):
  - students first package brownies and cookies into matching bags, and only
    then is the term "greatest common factor" named;
  - factors are shown as rectangles of squares;
  - the summary lists both sets of factors with the shared ones in bold, then
    says which shared factor is greatest;
  - teachers are warned that students miss factors.
- **OpenStax, *Prealgebra 2e*, sections 2.4, 2.5 and 10.6:**
  - each idea opens with a short story;
  - each worked example ends with a full-sentence answer and a check, and is
    followed at once by "Try It" questions in the same form;
  - methods are "How To" boxes of numbered commands;
  - factors are found by dividing in order, and come in pairs;
  - sections end with a self-check ("Confidently / With some help / No").
- **Math is Fun, "Greatest Common Factor":**
  - a three-step recipe that follows the name backwards: find the factors,
    circle the common ones, choose the greatest;
  - common factors shown as two labeled lists.
- **TERC EMPower** (adult numeracy curriculum):
  - keep the real situation central;
  - reasoning and pictures before rules;
  - several representations (pictures, tables, words, symbols).
