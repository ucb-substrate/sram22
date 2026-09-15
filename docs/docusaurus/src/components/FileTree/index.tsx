import React, { type JSX } from "react";
import styles from "./styles.module.css";

/**
 * Directory listing — the Starlight <FileTree> equivalent.
 *
 * Takes the tree as structured data rather than an indented markdown list:
 * MDX strips the leading whitespace from every line inside a JSX block (in
 * children and in attributes alike), so indentation cannot express nesting
 * here. A name ending in "/" renders as a directory.
 *
 *   <FileTree
 *     entries={[
 *       {name: "build/", children: [
 *         {name: "out.gds", comment: "the merged layout"},
 *       ]},
 *     ]}
 *   />
 */
export interface FileTreeEntry {
  name: string;
  comment?: string;
  children?: FileTreeEntry[];
}

function List({ entries }: { entries: FileTreeEntry[] }): JSX.Element {
  return (
    <ul className={styles.list}>
      {entries.map((e) => {
        const isDir = e.name.endsWith("/");
        return (
          <li className={isDir ? styles.dir : styles.file} key={e.name}>
            <span className={styles.row}>
              <span className={styles.icon} aria-hidden="true">
                {isDir ? "▸" : "•"}
              </span>
              <span className={styles.name}>{e.name}</span>
              {e.comment && <span className={styles.comment}>{e.comment}</span>}
            </span>
            {e.children?.length ? <List entries={e.children} /> : null}
          </li>
        );
      })}
    </ul>
  );
}

export default function FileTree({
  entries,
}: {
  entries: FileTreeEntry[];
}): JSX.Element {
  return (
    <div className={styles.tree}>
      <List entries={entries} />
    </div>
  );
}
