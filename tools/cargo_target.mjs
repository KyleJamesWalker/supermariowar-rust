// Where cargo writes binaries, which a worktree may share with the main checkout (tools/share-target.sh).
import { execFileSync } from 'node:child_process';

export function targetDir(root) {
  const meta = execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], { cwd: root, encoding: 'utf8' });
  return JSON.parse(meta).target_directory;
}
