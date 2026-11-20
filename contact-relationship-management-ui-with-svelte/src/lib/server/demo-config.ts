/** Whether the in-memory guided tour is explicitly enabled. */
export function demoTourEnabled(value: string | undefined): boolean {
  return value?.trim().toLowerCase() === "on";
}
