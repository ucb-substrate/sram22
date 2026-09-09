import React, { type JSX } from "react";
import config from "@site/src/data/config.json";
import styles from "./styles.module.css";

export default function ConfigTable(): JSX.Element {
  return (
    <div className={styles.cfg}>
      <div className={styles.tableWrap}>
        <table>
          <thead>
            <tr>
              <th>Option</th>
              <th>Type</th>
              <th>Required</th>
              <th>Purpose</th>
            </tr>
          </thead>
          <tbody>
            {config.options.map((o) => (
              <tr key={o.name}>
                <td>
                  <code>{o.name}</code>
                </td>
                <td>
                  <code>{o.type}</code>
                </td>
                <td>{o.required ? "yes" : "no"}</td>
                <td>{o.purpose}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h4>Validity constraints</h4>
      <ul>
        {config.constraints.map((c) => (
          <li key={c}>{c}</li>
        ))}
      </ul>

      <h4>Derived quantities</h4>
      <ul>
        {config.derived.map((d) => (
          <li key={d}>{d}</li>
        ))}
      </ul>
    </div>
  );
}
