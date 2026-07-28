import { useEffect, useRef, useState } from "react";
import type { NormalizedPullRequest } from "../../api/types.js";
import type { ProviderClient } from "../../providers/types.js";
import { hasActiveThread } from "../utils/commentStatus.js";
import { prKey } from "../utils/prKey.js";

export type CommentActivity = "active" | "resolved" | "error";

const CONCURRENCY = 6;

/**
 * Lazily fetches each PR's comment threads in the background (bounded
 * concurrency, so we don't fire off hundreds of requests at once on large
 * projects) to determine whether it has any unresolved thread. Used to
 * filter/mark PRs by whether they've been "actioned". Results are cached
 * per PR for the lifetime of the hook instance; a project switch or
 * refresh only fetches PRs not already seen.
 */
export function usePrCommentActivity(
  client: ProviderClient | null,
  project: string | undefined,
  pullRequests: NormalizedPullRequest[],
): Map<string, CommentActivity> {
  const [activity, setActivity] = useState<Map<string, CommentActivity>>(new Map());
  const requested = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!client || !project) return;
    let cancelled = false;

    const pending = pullRequests.filter((pr) => !requested.current.has(prKey(pr)));
    for (const pr of pending) requested.current.add(prKey(pr));
    if (pending.length === 0) return;

    let nextIndex = 0;
    const runWorker = async (): Promise<void> => {
      while (!cancelled) {
        const pr = pending[nextIndex++];
        if (!pr) return;
        let result: CommentActivity;
        try {
          const threads = await client.listPullRequestThreads(project, pr.repositoryId, pr.id);
          result = hasActiveThread(threads) ? "active" : "resolved";
        } catch {
          result = "error";
        }
        if (cancelled) return;
        const key = prKey(pr);
        setActivity((prev) => {
          const next = new Map(prev);
          next.set(key, result);
          return next;
        });
      }
    };

    const workerCount = Math.min(CONCURRENCY, pending.length);
    void Promise.all(Array.from({ length: workerCount }, () => runWorker()));

    return () => {
      cancelled = true;
    };
  }, [client, project, pullRequests]);

  return activity;
}
