// `page.data.title` convention (see `../../+layout.svelte`).
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = () => {
  return { title: "Kanban · Main X · CRM" };
};
