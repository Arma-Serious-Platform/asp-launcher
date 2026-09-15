export function sideColor(sideType?: string | null): string {
  switch (sideType?.toUpperCase()) {
    case "BLUE":
      return "text-sky-400";
    case "RED":
      return "text-red-400";
    case "GREEN":
      return "text-lime-400";
    default:
      return "text-zinc-200";
  }
}
