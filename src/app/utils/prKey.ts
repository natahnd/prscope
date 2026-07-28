export function prKey(pr: { repositoryId: string; id: number }): string {
  return `${pr.repositoryId}:${pr.id}`;
}
