import React, { type JSX } from "react";
import iface from "@site/src/data/interface.json";
import styles from "./styles.module.css";

const dirLabel: Record<string, string> = {
  input: "input",
  output: "output",
  inout: "inout",
};

export default function PinTable(): JSX.Element {
  return (
    <div className={styles.pinTable}>
      <table>
        <thead>
          <tr>
            <th>Pin</th>
            <th>Direction</th>
            <th>Width</th>
            <th>Layer</th>
            <th>Purpose</th>
          </tr>
        </thead>
        <tbody>
          {iface.pins.map((p) => (
            <tr key={p.name}>
              <td>
                <code>{p.name}</code>
              </td>
              <td>{dirLabel[p.direction]}</td>
              <td>
                <code>{p.width}</code>
              </td>
              <td>{p.layer}</td>
              <td>{p.purpose}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
