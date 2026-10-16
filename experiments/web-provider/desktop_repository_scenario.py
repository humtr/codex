"""Autonomous improvement of a tracked repository copy; originals stay read-only."""
import hashlib
import io
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile


class RepositoryDevelopment:
    timeout_seconds = 1200
    render_marker = "REPOSITORY_EVALUATION_DONE"

    def __init__(self, output, repository):
        self.output, self.repository = output, repository.resolve(strict=True)
        self.commands = []
        self.short_tmp = None

    def git(self, *args, cwd=None):
        return subprocess.check_output(['git', '-C', str(cwd or self.workspace), *args], stderr=subprocess.DEVNULL)

    def configure(self, config, root, home, workspace, marker):
        self.workspace = workspace
        self.head = self.git('rev-parse', 'HEAD', cwd=self.repository).decode().strip()
        names = self.git('ls-files', '-z', cwd=self.repository).decode().split('\0')
        self.protected = {n: hashlib.sha256((self.repository/n).read_bytes()).hexdigest() for n in names if n and (self.repository/n).is_file()}
        archive = self.git('archive', self.head, cwd=self.repository)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(workspace, filter='data')
        self.git('init')
        self.git('config', 'commit.gpgsign', 'false')
        self.git('config', 'core.hooksPath', '/dev/null')
        self.git('add', '.')
        self.git('-c', 'user.name=Owned evaluation', '-c', 'user.email=owned@example.invalid', 'commit', '-qm', 'Exact tracked source evaluation')
        # Avoid the already-reproduced Android Unix socket path overflow in the
        # repository's own nested fixtures. This is test-owned, not a live path.
        self.short_tmp = tempfile.TemporaryDirectory(prefix='l-', dir=os.environ['TMPDIR'])
        self.home = home
        for target in ['ai', 'clip']:
            dst = home/'.config'/target/'lib'
            dst.mkdir(parents=True, exist_ok=True)
            for p in (workspace/'lib').glob('*.py'):
                shutil.copyfile(p, dst/p.name)
        shutil.copytree(workspace/'config', home/'.config/ai/config')
        self.env = {'HOME':str(home), 'PREFIX':str(root/'p'), 'PATH':os.environ['PREFIX']+'/bin',
            'TMPDIR':self.short_tmp.name, 'SHELL':os.environ['PREFIX']+'/bin/bash', 'LANG':'C.UTF-8', 'PYTHONDONTWRITEBYTECODE':'1'}
        self.baseline = self.verify_repository('baseline-verify.log')
        assert self.baseline['accepted'], 'repository_baseline_failed_before_inference'
        return config

    def environment(self):
        return {'TMPDIR':self.short_tmp.name, 'PYTHONDONTWRITEBYTECODE':'1'}

    def verify_repository(self, name):
        with (self.output/name).open('w') as log:
            r = subprocess.run(['bash', 'verify/final-verify.sh'], cwd=self.workspace, env=self.env, stdout=log, stderr=log, timeout=180)
        text = (self.output/name).read_text()
        return {'exitCode':r.returncode, 'accepted':r.returncode==0 and '\nfail=0\n' in text}

    def task(self, _marker):
        return ('한국어로 소통하세요. 이 격리 저장소를 AGENTS.md와 제품 목적에 따라 검토하고 실제로 필요한 안정성 또는 기능 개선을 수행하세요. '
            '특정 수정 위치나 해결책은 미리 정하지 않았습니다. 근거 있는 문제를 스스로 찾아 분석·수정·회귀시험하세요. '
            '관련 문제가 더 있으면 첫 수정에서 멈추지 말고 이어서 처리하세요. 시험 실패가 생기면 판단하고 수정·재시험하세요. '
            '기존 기능과 사용자 데이터 보호를 유지하고 형식적인 변경이나 근거 없는 대규모 리팩터링은 하지 마세요. '
            '기존 전체 검증은 사전 통과했습니다. 수정 후 저장소의 전체 검증을 다시 수행하고, 변경 이유·실제 결과·남은 사항을 보고하세요. '
            '최초 보고에 LOCAL_INITIAL_REPORT, 유용한 중간보고 한 곳에 LOCAL_INTERMEDIATE_REPORT를 포함하세요. '
            '최종 보고는 REPOSITORY_EVALUATION_DONE으로 시작하고 실제 변경·검증을 설명하세요. '
            '원본이 아닌 추적 소스 사본이며 HOME·PREFIX·TMPDIR도 격리돼 있습니다. exec_command는 login=false를 사용하세요. '
            '이 작업 루트 밖의 프로젝트·계정·실서비스를 읽거나 변경하지 말고, 설치·로그인·배포·푸시·다른 모델 실행·하위 에이전트는 사용하지 마세요. '
            '준비된 HOME과 TMPDIR를 유지하고 최대20분 안에 가능한 작업을 스스로 이어가세요.')

    def observe(self, row):
        item = row.get('params', {}).get('item', {})
        if row.get('method')=='item/completed' and item.get('type')=='commandExecution':
            self.commands.append({'exitCode':item.get('exitCode'), 'command':item.get('command','')})
            (self.output/'repository-commands.json').write_text(json.dumps(self.commands, indent=2)+'\n')
        if row.get('method') in ['item/completed','turn/completed'] and self.workspace.exists():
            (self.output/'changes.patch').write_bytes(self.git('diff', '--binary'))

    def respond(self, _row, _send):
        return False

    def reply_matches(self, text):
        return text.strip().startswith('REPOSITORY_EVALUATION_DONE')

    def initial_acceptance(self, report):
        return report['turnCompleted'] and report['nativeReplyMatched'] and report['successfulShellCalls']>=2

    @staticmethod
    def reports_accepted(evidence):
        # Autonomous work can report several discoveries, unlike the fixed
        # surface fixture. Preserve every report instead of enforcing one step.
        return (sum(item['initial'] for item in evidence)==1
            and sum(item['intermediate'] for item in evidence)>=1
            and sum(item['final'] for item in evidence)==1)

    def verify(self, root, home, workspace, items, report):
        result = {'sourceHead':self.head, 'baselineFullVerifyPassed':self.baseline['accepted'],
            'fullVerify':self.verify_repository('final-verify.log'),
            'originalTrackedFilesUnchanged':all(hashlib.sha256((self.repository/n).read_bytes()).hexdigest()==v for n,v in self.protected.items()),
            'completedCommands':len(self.commands)}
        diff = self.git('diff', '--name-only').decode().splitlines()
        result['changedFiles'] = diff
        result['implementationChanged'] = any(n.startswith(('lib/','bin/','code/')) for n in diff)
        result['modelRanRepositoryVerification'] = any(re.search(r'\bbash\s+(?:\./)?verify/final-verify\.sh\b',c['command']) and c['exitCode']==0 for c in self.commands)
        (self.output/'changes.patch').write_bytes(self.git('diff', '--binary'))
        result['accepted'] = result['baselineFullVerifyPassed'] and result['fullVerify']['accepted'] and result['originalTrackedFilesUnchanged'] and result['implementationChanged'] and result['modelRanRepositoryVerification']
        (self.output/'repository-result.json').write_text(json.dumps(result, indent=2)+'\n')
        return result

    def cleanup(self):
        if self.short_tmp:
            self.short_tmp.cleanup()
