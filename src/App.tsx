import Launcher from "./Launcher";
import Browser from "./Browser";

/**
 * Both windows load the same bundle; the query string picks the view.
 * `open_main_window` (commands.rs) builds the browser window with
 * `index.html?view=browser`.
 */
export default function App() {
  const view = new URLSearchParams(window.location.search).get("view");
  return view === "browser" ? <Browser /> : <Launcher />;
}
