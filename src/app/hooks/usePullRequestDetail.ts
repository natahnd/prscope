import { useEffect, useRef, useState } from "react";
import type { NormalizedPullRequest, PrThread, PullRequestDetail } from "../../api/types.js";
import type { ProviderClient } from "../../providers/types.js";

export interface PullRequestDetailState {
  detail?: PullRequestDetail;
  threads: PrThread[];
  loading: boolean;
  error?: string;
}

const EMPTY_STATE: PullRequestDetailState = { threads: [], loading: false };

export function usePullRequestDetail(
  client: ProviderClient | null,
  project: string | undefined,
  pr: NormalizedPullRequest | undefined,
): PullRequestDetailState {
  const cache = useRef<Map<string, PullRequestDetailState>>(new Map());
  const [state, setState] = useState<PullRequestDetailState>(EMPTY_STATE);

  useEffect(() => {
    if (!client || !project || !pr) {
      setState(EMPTY_STATE);
      return;
    }

    const key = `${pr.repositoryId}:${pr.id}`;
    const cached = cache.current.get(key);
    if (cached) {
      setState(cached);
      return;
    }

    let cancelled = false;
    setState({ ...EMPTY_STATE, loading: true });

    void (async () => {
      try {
        const [detail, threads] = await Promise.all([
          client.getPullRequest(project, pr.repositoryId, pr.id),
          client.listPullRequestThreads(project, pr.repositoryId, pr.id),
        ]);
        if (cancelled) return;

        const result: PullRequestDetailState = { detail, threads, loading: false };
        cache.current.set(key, result);
        setState(result);
      } catch (error) {
        if (cancelled) return;
        const result: PullRequestDetailState = {
          ...EMPTY_STATE,
          error: error instanceof Error ? error.message : String(error),
        };
        cache.current.set(key, result);
        setState(result);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [client, project, pr]);

  return state;
}
