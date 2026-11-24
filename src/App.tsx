import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [version, setVersion] = useState("…");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<string>("restic_version")
      .then(setVersion)
      .catch((e: unknown) => setError(String(e)));
  }, []);

  return (
    <main style={{ fontFamily: "system-ui", padding: 16 }}>
      <h1 style={{ margin: "0 0 12px", fontSize: 18 }}>restarilo</h1>
      {error ? <pre style={{ color: "crimson" }}>{error}</pre> : <pre>{version}</pre>}
    </main>
  );
}

export default App;
