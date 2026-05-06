/**
 * Count words in a plain-text string.
 * Used to compute word_count before sending a scene_save IPC call.
 */
export function countWords(text: string): number {
  return text.trim() === "" ? 0 : text.trim().split(/\s+/).length;
}

/** Count characters (Unicode code points, not bytes). */
export function countChars(text: string): number {
  return [...text].length;
}
