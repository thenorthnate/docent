import type { ChangedFile } from "@/lib/review";

/** A line of the changed-files tree: a folder, or a file when `file` is set. Keys are unique, even for files whose names read the same. */
export type FileTreeRow = { key: string; name: string; depth: number; file: ChangedFile | null };

type Folder = { folders: Map<string, Folder>; files: ChangedFile[] };

const newFolder = (): Folder => ({ folders: new Map(), files: [] });

/** The last part of a path. */
export function fileName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/**
 * `files` as a tree, from the top down: folders before files, each in order.
 * A folder that holds only one other folder is joined with it.
 */
export function fileTreeRows(files: ChangedFile[]): FileTreeRow[] {
  const root = newFolder();
  const order = new Map(files.map((file, index) => [file, index]));
  for (const file of files) {
    let folder = root;
    for (const name of file.path.split("/").slice(0, -1)) {
      const next = folder.folders.get(name) ?? newFolder();
      folder.folders.set(name, next);
      folder = next;
    }
    folder.files.push(file);
  }
  return rowsIn(root, "", 0, order);
}

function rowsIn(parent: Folder, path: string, depth: number, order: Map<ChangedFile, number>): FileTreeRow[] {
  const rows: FileTreeRow[] = [];
  for (const [first, child] of inOrder([...parent.folders], ([name]) => name)) {
    let [name, folder] = [first, child];
    while (folder.files.length === 0 && folder.folders.size === 1) {
      const [[next, only]] = folder.folders;
      name += `/${next}`;
      folder = only;
    }
    const key = `${path}${name}/`;
    rows.push({ key, name, depth, file: null }, ...rowsIn(folder, key, depth + 1, order));
  }
  for (const file of inOrder(parent.files, (file) => file.path)) {
    // Folder keys end in `/`, which a file key never does.
    rows.push({ key: `${order.get(file)}:${file.path}`, name: fileName(file.path), depth, file });
  }
  return rows;
}

/** Sorted by plain string order, whatever the locale. */
function inOrder<T>(items: T[], by: (item: T) => string): T[] {
  return [...items].sort((a, b) => (by(a) < by(b) ? -1 : 1));
}
