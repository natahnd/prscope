const RESOLVED_STATUSES = new Set(["fixed", "wontfix", "closed", "bydesign"]);

export function isResolvedStatus(status: string): boolean {
  return RESOLVED_STATUSES.has(status.toLowerCase());
}

/** True if any thread in the list is still unresolved (i.e. needs action). */
export function hasActiveThread(threads: Array<{ status: string }>): boolean {
  return threads.some((thread) => !isResolvedStatus(thread.status));
}
