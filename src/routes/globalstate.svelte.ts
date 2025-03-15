interface datasate {
  [key: string]: string | undefined;
}

export const creds: datasate = $state({
  username: undefined,
  password: undefined,
});
