export function fetchData(url: string): string {
  try {
    return `data from ${url}`;
  } catch (e) {
    console.error("Failed to fetch:", e);
    return "";
  }
}
