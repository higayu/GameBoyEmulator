import { useEffect, useState } from "react";

import init, {
  get_version,
  inspect_rom,
} from "./wasm/emulator_wasm.js";

import "./App.css";

function App() {
  const [ready, setReady] = useState(false);
  const [version, setVersion] = useState("");
  const [romInfo, setRomInfo] = useState(null);
  const [fileName, setFileName] = useState("");
  const [error, setError] = useState("");

  // WebAssemblyの初期化
  useEffect(() => {
    async function initialize() {
      try {
        await init();

        setVersion(get_version());
        setReady(true);
      } catch (error) {
        setError(String(error));
      }
    }

    initialize();
  }, []);

  // ROMファイルの読み込み
  async function handleRomChange(event) {
    const file = event.target.files?.[0];

    if (!file || !ready) {
      return;
    }

    setError("");
    setRomInfo(null);
    setFileName(file.name);

    try {
      // ROMをバイナリデータとして取得
      const buffer = await file.arrayBuffer();
      const bytes = new Uint8Array(buffer);

      // Rust側でROMヘッダーを解析
      const result = inspect_rom(bytes);

      setRomInfo(result);
    } catch (error) {
      setError(String(error));
    } finally {
      // 同じファイルを再選択できるようにする
      event.target.value = "";
    }
  }

  return (
    <main className="app">
      <header className="header">
        <h1>GB Emulator</h1>
        <span>Version {version || "---"}</span>
      </header>

      <div className="toolbar">
        <label className="file-button">
          ROMを開く

          <input
            type="file"
            accept=".gb,.gbc"
            disabled={!ready}
            onChange={handleRomChange}
          />
        </label>

        <span className={ready ? "online" : "offline"}>
          {ready ? "WASM Ready" : "Initializing..."}
        </span>
      </div>

      <section className="workspace">
        <div className="game-panel">
          <canvas width={160} height={144} />
          <p>160 × 144 pixels</p>
        </div>

        <aside className="info-panel">
          <h2>ROM Information</h2>

          {!romInfo && (
            <p>ROMファイルを選択してください。</p>
          )}

          {romInfo && (
            <dl>
              <dt>ファイル名</dt>
              <dd>{fileName}</dd>

              <dt>ゲームタイトル</dt>
              <dd>{romInfo.title}</dd>

              <dt>対応ハードウェア</dt>
              <dd>{romInfo.hardware}</dd>

              <dt>カートリッジ</dt>
              <dd>{romInfo.cartridge_type}</dd>

              <dt>カートリッジタイプコード</dt>
              <dd>
                0x{romInfo.cartridge_type_code
                  .toString(16)
                  .padStart(2, "0")
                  .toUpperCase()}
              </dd>

              <dt>ROM容量</dt>
              <dd>{romInfo.rom_size / 1024} KB</dd>

              <dt>ROMサイズコード</dt>
              <dd>
                0x{romInfo.rom_size_code
                  .toString(16)
                  .padStart(2, "0")
                  .toUpperCase()}
              </dd>

              <dt>RAMサイズコード</dt>
              <dd>
                0x{romInfo.ram_size_code
                  .toString(16)
                  .padStart(2, "0")
                  .toUpperCase()}
              </dd>

              <dt>ヘッダーチェックサム</dt>
              <dd>
                {romInfo.header_checksum_valid
                  ? "正常"
                  : "不一致"}
              </dd>
            </dl>
          )}

          {error && (
            <p className="error">{error}</p>
          )}
        </aside>
      </section>
    </main>
  );
}

export default App;