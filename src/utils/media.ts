/** Same rule as the backend (`valid_icon_name`): a plain picture file name, nothing else. */
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
 * Where an item's picture lives: the catalog stores only the file name and the site loads it
 * straight from its `public/art/items` folder. Older values that are already a site path or an
 * https URL pass through.
 */
export function itemIconUrl(icon: string): string {
  if (icon.startsWith('/') || icon.startsWith('https://')) return icon
  return `/art/items/${encodeURIComponent(icon)}`
}

/** Longest side, in pixels, of an uploaded racer photo (shown at up to ~100 px, so sharp on 2x-4x screens). */
export const PHOTO_MAX_SIDE = 512

/**
 * Shrinks a picture in the browser before uploading it, keeping its format, so a 4000 px photo from a
 * phone never travels or gets served. Anything already small enough is sent untouched.
 */
export async function shrinkPicture(file: File, maxSide = PHOTO_MAX_SIDE): Promise<Blob> {
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
