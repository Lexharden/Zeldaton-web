export type Wave = 'sine' | 'square' | 'sawtooth' | 'triangle'

/** A note: frequency in Hz (0 = silence) and length in seconds. */
export type Note = [freq: number, seconds: number]

/**
 * Plays a short melody on a soft ocarina-like synth (sine + vibrato, no audio files). Silent when
 * the browser has no Web Audio or blocks it: sound is always a bonus, never required.
 */
export function playNotes(notes: Note[], opts: { type?: Wave; volume?: number } = {}) {
  try {
    const Ctx =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
    if (!Ctx) return
    const ac = new Ctx()
    const out = ac.createGain()
    out.gain.value = opts.volume ?? 0.16
    out.connect(ac.destination)
    let at = ac.currentTime + 0.05
    for (const [freq, dur] of notes) {
      if (freq > 0) {
        const osc = ac.createOscillator()
        const env = ac.createGain()
        const vibrato = ac.createOscillator()
        const depth = ac.createGain()
        osc.type = opts.type ?? 'sine'
        osc.frequency.value = freq
        vibrato.frequency.value = 5.5
        depth.gain.value = freq * 0.006
        vibrato.connect(depth).connect(osc.frequency)
        env.gain.setValueAtTime(0, at)
        env.gain.linearRampToValueAtTime(1, at + Math.min(0.05, dur / 4))
        env.gain.setValueAtTime(1, at + dur * 0.75)
        env.gain.linearRampToValueAtTime(0, at + dur)
        osc.connect(env).connect(out)
        osc.start(at)
        vibrato.start(at)
        osc.stop(at + dur + 0.02)
        vibrato.stop(at + dur + 0.02)
      }
      at += dur
    }
    setTimeout(() => void ac.close(), (at - ac.currentTime + 0.5) * 1000)
  } catch {
    // Audio is a bonus.
  }
}

// Pitches used by the songs (equal temperament, A4 = 440).
export const P = {
  D3: 146.83,
  A3: 220,
  D4: 293.66,
  F4: 349.23,
  A4: 440,
  B4: 493.88,
  C5: 523.25,
  D5: 587.33,
  E5: 659.25,
  F5: 698.46,
  G5: 783.99,
} as const
