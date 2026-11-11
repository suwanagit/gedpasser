import { MathText } from './MathText'

type Block = { kind: 'p'; lines: string[] } | { kind: 'ul'; items: string[] }

/**
 * Lesson format: paragraphs separated by blank lines; lines starting with
 * "- " are bullet points; math between $...$ (rendered by MathText).
 */
export function parseLesson(text: string): Block[] {
  const blocks: Block[] = []
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trimEnd()
    const last = blocks[blocks.length - 1]
    if (line.trim() === '') {
      blocks.push({ kind: 'p', lines: [] }) // paragraph break
    } else if (line.startsWith('- ')) {
      if (last?.kind === 'ul') last.items.push(line.slice(2))
      else blocks.push({ kind: 'ul', items: [line.slice(2)] })
    } else if (last?.kind === 'p') {
      last.lines.push(line)
    } else {
      blocks.push({ kind: 'p', lines: [line] })
    }
  }
  return blocks.filter((b) => (b.kind === 'p' ? b.lines.length > 0 : b.items.length > 0))
}

export function LessonText({ text }: { text: string }) {
  return (
    <div className="lesson-text">
      {parseLesson(text).map((b, i) =>
        b.kind === 'ul' ? (
          <ul key={i}>
            {b.items.map((item, j) => (
              <li key={j}>
                <MathText text={item} />
              </li>
            ))}
          </ul>
        ) : (
          <p key={i}>
            <MathText text={b.lines.join('\n')} />
          </p>
        ),
      )}
    </div>
  )
}
