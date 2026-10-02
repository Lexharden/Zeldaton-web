import { runtime } from '@/config/runtime'

/** Same rule as the backend (`media::valid_name`): a plain picture file name, nothing else. */
export function validIconName(name: string): boolean {
  return (
    name.length > 0 &&
    [...name].length <= 120 &&
    !name.startsWith('.') &&
    !name.includes('..') &&
    /^[\p{L}\p{N}_\-.'() ]+$/u.test(name) &&
    /\.(png|jpe?g|webp)$/i.test(name)
  )
}

/**
 * Where an item's picture lives. A catalog item stores only the file name; the backend serves it
 * from `/api/media/items/<name>`. Without a backend (mock mode) the files shipped in
 * `public/art/items` are used. Older values that are already a site path or an https URL pass through.
 */
export function itemIconUrl(icon: string): string {
  if (icon.startsWith('/') || icon.startsWith('https://')) return icon
  const name = encodeURIComponent(icon)
  return runtime.mockMode ? `/art/items/${name}` : `${runtime.apiUrl || '/api'}/media/items/${name}`
}

/** Longest side, in pixels, of a picture kept in the library (shown at 32-64 px, so 2x-4x sharp). */
export const ICON_MAX_SIDE = 256

/**
 * Shrinks a picture in the browser before uploading it, keeping its name and format, so a 4000 px
 * render never travels or gets served. Anything already small enough is sent untouched.
 */
export async function shrinkPicture(file: File, maxSide = ICON_MAX_SIDE): Promise<Blob> {
  if (typeof createImageBitmap !== 'function' || typeof document === 'undefined') return file
  let bitmap: ImageBitmap
  try {
    bitmap = await createImageBitmap(file)
  } catch {
    return file
  }
  const scale = maxSide / Math.max(bitmap.width, bitmap.height)
  if (scale >= 1) {
    bitmap.close()
    return file
  }
  const canvas = document.createElement('canvas')
  canvas.width = Math.max(1, Math.round(bitmap.width * scale))
  canvas.height = Math.max(1, Math.round(bitmap.height * scale))
  const ctx = canvas.getContext('2d')
  if (!ctx) {
    bitmap.close()
    return file
  }
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height)
  bitmap.close()
  const type = file.type === 'image/jpeg' || file.type === 'image/webp' ? file.type : 'image/png'
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, type, 0.9))
  return blob && blob.size < file.size ? blob : file
}
