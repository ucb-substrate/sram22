import React, { useMemo, useState, type JSX } from "react";
import macros from "@site/src/data/macros.json";
import styles from "./styles.module.css";

export default function MacroTable(): JSX.Element {
  const [validatedOnly, setValidatedOnly] = useState(false);

  const sorted = useMemo(
    () =>
      [...macros.macros].sort(
        (a, b) => a.num_words - b.num_words || a.data_width - b.data_width,
      ),
    [],
  );
  const validated = sorted.filter((m) => m.silicon_validated).length;
  const rows = validatedOnly
    ? sorted.filter((m) => m.silicon_validated)
    : sorted;

  return (
    <div className={styles.mt}>
      <div className={styles.bar}>
        <span className={styles.count}>
          {sorted.length} macros · {validated} silicon-validated
        </span>
        <label className={styles.filter}>
          <input
            type="checkbox"
            checked={validatedOnly}
            onChange={(e) => setValidatedOnly(e.target.checked)}
          />{" "}
          Silicon-validated only
        </label>
      </div>
      <div className={styles.wrap}>
        <table>
          <thead>
            <tr>
              <th>Macro</th>
              <th>Words</th>
              <th>Width</th>
              <th>Mux</th>
              <th>Write&nbsp;size</th>
              <th>Capacity</th>
              <th>Silicon</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((m) => (
              <tr key={m.name}>
                <td>
                  <a href={`${macros.repo}/tree/master/${m.name}`}><code>{m.name}</code></a>
                </td>
                <td>{m.num_words}</td>
                <td>{m.data_width}</td>
                <td>{m.mux_ratio}</td>
                <td>{m.write_size}</td>
                <td>{m.capacity_kib} KiB</td>
                <td>
                  {m.silicon_validated ? (
                    <span className={styles.yes}>✓</span>
                  ) : (
                    "—"
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
