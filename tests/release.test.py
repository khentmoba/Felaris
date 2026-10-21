"""Release guard tests; packaging and publication are always mocked."""
import contextlib
import base64
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('release', ROOT / 'scripts/build.py')
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)
notify_spec = importlib.util.spec_from_file_location('notify', ROOT / 'scripts/notify_discord.py')
notify = importlib.util.module_from_spec(notify_spec)
notify_spec.loader.exec_module(notify)
SHA = 'a' * 40


class NotificationTests(unittest.TestCase):
    def test_payload_matches_delivery_and_normalizes_bullets(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / 'felaris.exe'
            binary.write_bytes(b'fixture')
            for upload in (True, False):
                arguments = ['notify', '--version', '1.2.3', '--summary', '* Fixed hover\n- Added prompts', '--exe', str(binary)]
                if not upload:
                    arguments.append('--no-upload')
                with patch('sys.argv', arguments), patch.object(notify, 'get_webhook_url', return_value='https://example.test/webhook'), \
                        patch.object(notify, 'send_webhook', return_value={'id': '123'}) as send, \
                        contextlib.redirect_stdout(io.StringIO()):
                    notify.main()
                payload = send.call_args.args[1]
                self.assertEqual(payload['allowed_mentions'], {'parse': []})
                embed = payload['embeds'][0]
                self.assertEqual(embed['color'], 4363994)
                self.assertEqual(embed['title'], 'Release: Felaris v1.2.3')
                self.assertEqual(embed['fields'][-1]['value'], '- Fixed hover\n- Added prompts')
                self.assertEqual(embed['fields'][2]['name'], 'Attached File' if upload else 'Download')
                self.assertEqual(send.call_args.kwargs['file_path'], binary if upload else None)
                self.assertIn('attached' if upload else 'link below', payload['content'])

    def test_webhook_requests_message_receipt(self):
        response = io.BytesIO(b'{"id":"123"}')
        with patch.object(notify, 'urlopen', return_value=response) as post:
            self.assertEqual(notify.send_webhook('https://example.test/webhook?wait=false', {'content': 'test'}), {'id': '123'})
        self.assertEqual(post.call_args.args[0].full_url, 'https://example.test/webhook?wait=true')


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / 'Cargo.toml').write_text('[package]\nversion = "1.2.3"\n', encoding='utf-8')
        self.manifest = self.root / 'version.json'
        self.manifest.write_text(json.dumps({
            'version': '1.2.3',
            'release_url': 'https://github.com/khentmoba/Felaris/releases/tag/v1.2.3',
            'download_url': 'https://github.com/khentmoba/Felaris/releases/download/v1.2.3/felaris.exe',
            'summary': 'Committed notes',
        }), encoding='utf-8')
        self.commands = []
        self.dirty = ''
        self.head = SHA
        self.remote = SHA
        self.tag = ''
        self.ci = [{'headSha': SHA, 'status': 'completed', 'conclusion': 'success'}]
        self.fail = None
        self.change_during_build = False
        self.missing_asset = False

    def run_command(self, command, **kwargs):
        self.commands.append(command)
        result = ''
        if command[:2] == ['git', 'status']:
            result = self.dirty
        elif command[:2] == ['git', 'rev-parse']:
            result = self.head
        elif command[:3] == ['git', 'ls-remote', '--tags']:
            result = self.tag
        elif command[:2] == ['git', 'ls-remote']:
            result = self.remote + '\trefs/heads/main'
        elif command[:3] == ['gh', 'run', 'list']:
            result = json.dumps(self.ci)
        else:
            stage = 'github' if command[0] == 'gh' else 'notify' if command[0] == sys.executable else 'package'
            if stage == self.fail:
                if kwargs.get('check'):
                    raise subprocess.CalledProcessError(7, command)
                return SimpleNamespace(returncode=7)
            if stage == 'package':
                directory = self.root / 'target/release'
                directory.mkdir(parents=True)
                for name in ('felaris.exe', 'felaris-setup.exe', 'SHA256SUMS'):
                    if not self.missing_asset or name != 'SHA256SUMS':
                        (directory / name).write_bytes(b'fixture')
                if self.change_during_build:
                    self.head = 'b' * 40
        return SimpleNamespace(stdout=result, returncode=0)

    def invoke(self, *arguments, gh='gh'):
        with patch.object(release, 'PROJECT_ROOT', self.root), patch('sys.argv', ['build.py', *arguments]), \
                patch('subprocess.run', side_effect=self.run_command), patch('shutil.which', return_value=gh), \
                contextlib.redirect_stdout(io.StringIO()):
            release.main()

    def published(self):
        return [command for command in self.commands if command[:3] == ['gh', 'release', 'create'] or command[0] == sys.executable]

    def test_github_failure_stops_discord(self):
        self.fail = 'github'
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke()
        self.assertEqual(len(self.published()), 1)

    def test_package_failure_stops_publication(self):
        self.fail = 'package'
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke()
        self.assertEqual(self.published(), [])

    def test_publishing_preserves_metadata_and_targets_built_commit(self):
        before = self.manifest.read_bytes()
        self.invoke()
        self.assertEqual(self.manifest.read_bytes(), before)
        github, notification = self.published()
        self.assertEqual(github[github.index('--target') + 1], SHA)
        self.assertIn('Committed notes', notification)

    def test_prepare_only_writes_metadata(self):
        self.invoke('--prepare', '--summary', 'New notes')
        self.assertEqual(json.loads(self.manifest.read_text())['summary'], 'New notes')
        self.assertEqual(self.commands, [])

    def test_unprepared_summary_rejected_before_build(self):
        with self.assertRaisesRegex(ValueError, '--summary differs'):
            self.invoke('--summary', 'Uncommitted notes')
        self.assertEqual(self.commands, [])

    def test_dirty_checkout_rejected(self):
        self.dirty = ' M version.json'
        with self.assertRaisesRegex(ValueError, 'clean'):
            self.invoke()
        self.assertEqual(self.published(), [])

    def test_unmerged_commit_rejected(self):
        self.remote = 'b' * 40
        with self.assertRaisesRegex(ValueError, 'origin/main'):
            self.invoke()
        self.assertEqual(self.published(), [])

    def test_existing_tag_rejected(self):
        self.tag = SHA + '\trefs/tags/v1.2.3'
        with self.assertRaisesRegex(ValueError, 'already exists'):
            self.invoke()
        self.assertEqual(self.published(), [])

    def test_missing_github_cli_requires_explicit_skip(self):
        with self.assertRaisesRegex(ValueError, 'required'):
            self.invoke(gh=None)
        self.invoke('--skip-github', gh=None)
        self.assertEqual(len(self.published()), 1)
        self.assertEqual(self.published()[0][0], sys.executable)

    def test_changed_source_after_build_rejected(self):
        self.change_during_build = True
        with self.assertRaisesRegex(ValueError, 'changed during'):
            self.invoke()
        self.assertEqual(self.published(), [])

    def test_missing_verified_asset_rejected(self):
        self.missing_asset = True
        with self.assertRaisesRegex(ValueError, 'Missing verified'):
            self.invoke()
        self.assertEqual(self.published(), [])

    def test_dry_run_never_builds_or_changes_metadata(self):
        before = self.manifest.read_bytes()
        self.invoke('--dry-run')
        self.assertTrue(all(command[0] == 'git' or command[:3] == ['gh', 'run', 'list'] for command in self.commands))
        self.assertEqual(self.manifest.read_bytes(), before)

    def test_mismatched_version_rejected(self):
        (self.root / 'Cargo.toml').write_text('[package]\nversion = "2.0.0"\n', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'match Cargo.toml'):
            self.invoke()

    def test_ci_must_pass_on_the_exact_commit(self):
        for runs in ([], [{'headSha': SHA, 'status': 'in_progress', 'conclusion': ''}],
                     [{'headSha': SHA, 'status': 'completed', 'conclusion': 'failure'}],
                     [{'headSha': 'b' * 40, 'status': 'completed', 'conclusion': 'success'}]):
            with self.subTest(runs=runs):
                self.ci = runs
                with self.assertRaisesRegex(ValueError, 'CI must pass'):
                    self.invoke()
                self.assertEqual(self.published(), [])

    @unittest.skipUnless(sys.platform == 'win32', 'PowerShell entry point is Windows-only')
    def test_powershell_wrapper_forwards_arguments_and_failure(self):
        script = str(ROOT / 'scripts/build.ps1').replace("'", "''")
        command = ("function python { ConvertTo-Json -InputObject @($args) -Compress; $global:LASTEXITCODE = 37 }; "
                   f"& '{script}' -Summary 'Release notes with spaces' -Prepare -SkipGitHub -SkipDiscordNotify -DryRun; exit $LASTEXITCODE")
        encoded = base64.b64encode(command.encode('utf-16le')).decode('ascii')
        result = subprocess.run(['powershell', '-NoProfile', '-EncodedCommand', encoded], capture_output=True, text=True)
        self.assertEqual(result.returncode, 37, result.stderr)
        self.assertEqual(json.loads(result.stdout), [str(ROOT / 'scripts/build.py'), '--summary',
                         'Release notes with spaces', '--prepare', '--skip-discord', '--skip-github', '--dry-run'])

    def test_windows_icons_match_canonical_branding(self):
        result = subprocess.run([sys.executable, str(ROOT / 'scripts/generate-icons.py'), '--check'],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('match', result.stdout)


if __name__ == '__main__':
    unittest.main()
