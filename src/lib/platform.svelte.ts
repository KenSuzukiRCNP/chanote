// Where the app is running. Filled from app_info at boot; drives the mobile layout and
// the file flows (save dialog vs. Documents folder, drag & drop vs. picker only).
export const platform = $state({
  os: "unknown" as string,   // macos | windows | linux | ios | android | unknown
  mobile: false,
  exportsDir: "",
});

export const isIOS = () => platform.os === "ios";
export const isAndroid = () => platform.os === "android";
