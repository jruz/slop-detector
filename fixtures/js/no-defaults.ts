export function getRepo(): string {
  const repo = process.env["CI_REPO"] ?? "jruz/slop-detector";
  return repo;
}
