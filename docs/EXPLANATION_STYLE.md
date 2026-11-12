# Explanation style

This app is a practice playground, not a school. Learners test themselves
first. An explanation appears only when they ask ("Show me how"), so it
has to be short, clear, and enough to unstick them. For more, the app
points them to a topic and free pages to study (`content/study.toml`).

This guide covers every question's `explanation` and `common_mistake`, and
the Quick review cards. It carries over what worked when we tried full
lessons (see "Where this comes from").

## Write it like a good teacher at the board

- **One line per step, written like work on paper.** The app shows each
  line of the explanation on its own row, so put each step on its own
  line.
  ```
  Let h be the hours: $75 + 30h = 255$.
  Subtract the \$75 fee: $30h = 180$.
  Divide by 30: $h = 6$. So Ken rented the skiff for **6 hours**.
  ```
- **Say why before how**, in a few words: "Subtract the \$75 fee", not
  "Subtract 75".
- **Show where every number comes from.** If 38 becomes 40 − 2, say so.
  Never state a result without the work that gets it.
- **End with the answer in a sentence that restates the question,** in
  bold: "So the GCF of 12 and 18 is **6**." Add a quick check when one fits
  ("Check: $12 \div 6 = 2$ and $18 \div 6 = 3$.").
- **Bold what to notice:** the shared factors, the answer.
- **Talk to the reader:** "you", "let's". Point things out: "Notice that…".
- **About 6th-grade reading level.** Short sentences, everyday words. Never
  write "simply", "obviously" or "just". No talking down; adults bring
  real experience.
- **Keep it short.** Most explanations are 2 to 4 lines. If it needs a
  whole lesson, it needs a study link instead.

## The common mistake

One sentence naming the mistake, showing the wrong result where it helps:
"Adding 7 instead of subtracting it: 15 + 7 = 22." It appears as "Watch
out" under the explanation.

## Study links

`content/study.toml` gives each principle a search topic (a few plain
words) and one to three free pages. Use OpenStax textbooks (*Prealgebra 2e*,
*Elementary Algebra 2e*, *Intermediate Algebra 2e*, *Introductory
Statistics 2e*) and Math is Fun. Every link must open when added. The file
is reviewed as a whole (`cargo content-template study`, then
`cargo content-approve study`).

## Where this comes from

Read in November 2026. Our text is in our own words; nothing is copied.

- **Illustrative Mathematics, Grade 6, Lesson 16 "Common Factors":**
  - a real problem comes before the new word;
  - worked examples are labeled lists with what matters in bold;
  - students are reminded to find every factor.
- **OpenStax, *Prealgebra 2e*** (sections 2.4, 2.5, 10.6):
  - every worked example ends with a full-sentence answer and a check;
  - methods are written as short numbered commands.
- **Math is Fun, "Greatest Common Factor":** short steps named after the
  idea itself.
- **TERC EMPower** (adult numeracy):
  - keep the real situation central;
  - put reasoning before rules.
