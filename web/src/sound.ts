// Short tones made in the browser (no audio files), plus a phone buzz.

const KEY = 'ged.sound'
let ctx: AudioContext | null = null

export function soundOn(): boolean {
  try {
    return localStorage.getItem(KEY) !== '0'
  } catch {
    return true
  }
}

export function setSoundOn(on: boolean) {
  try {
    localStorage.setItem(KEY, on ? '1' : '0')
  } catch {
    // private mode: stays on for this visit
  }
}

function tone(freq: number, start: number, length: number, type: OscillatorType = 'sine') {
  if (!ctx) return
  const osc = ctx.createOscillator()
  const gain = ctx.createGain()
  osc.type = type
  osc.frequency.value = freq
  const t = ctx.currentTime + start
  gain.gain.setValueAtTime(0.0001, t)
  gain.gain.exponentialRampToValueAtTime(0.16, t + 0.015)
  gain.gain.exponentialRampToValueAtTime(0.0001, t + length)
  osc.connect(gain).connect(ctx.destination)
  osc.start(t)
  osc.stop(t + length + 0.02)
}

export function play(kind: 'right' | 'wrong' | 'done') {
  if (!soundOn()) return
  try {
    ctx ??= new AudioContext()
    void ctx.resume()
    if (kind === 'right') {
      tone(660, 0, 0.12)
      tone(990, 0.09, 0.2)
    } else if (kind === 'wrong') {
      tone(220, 0, 0.22, 'triangle')
      navigator.vibrate?.(60)
    } else {
      ;[523, 659, 784, 1047].forEach((f, i) => tone(f, i * 0.1, 0.25))
    }
  } catch {
    // no audio available: stay silent
  }
}
