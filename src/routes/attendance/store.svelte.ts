interface datasate {
  [key: string]: string | undefined;
}
interface datavec {
  [key: string]: [string];
}
export const selsemid: datasate = $state({ value: undefined });
export const loading: datavec = $state({ value: [""] });
