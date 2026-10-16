"""Short real Core/app generation proof; no compaction or repository reevaluation."""


class Generation:
    timeout_seconds=240

    def __init__(self):
        self.commands=0
        self.checks={}

    def configure(self,config,root,home,workspace,marker):
        self.workspace,self.marker=workspace,marker
        (workspace/'seed.txt').write_text(marker+'\n')
        return config

    def task(self,_marker):
        return ('This is a bounded authorized disposable test. Say LOCAL_INITIAL_REPORT. '
            'Read seed.txt using one exec_command with login=false. Remember its actual content. '
            'Say LOCAL_INTERMEDIATE_REPORT, then use a second exec_command with login=false '
            'to write exactly that content plus one newline to proof.txt. '
            'Final answer exactly the actual seed value. No other tools, files or work.')

    def observe(self,row):
        item=row.get('params',{}).get('item',{})
        if row.get('method')=='item/completed' and item.get('type')=='commandExecution':self.commands+=1

    def respond(self,_row,_send):return False

    def verify_cold(self,rpc,collect,thread,deadline,report,events):
        assert self.commands==2,'initial_tool_count_not_exact'
        before=(self.workspace/'proof.txt').stat()
        self.checks.update(self.restart_backend())
        events.clear()
        rpc(103,'turn/start',{'threadId':thread,'input':[{'type':'text','text':
            'Recall the actual seed value from the completed conversation. Reply exactly that value. '
            'Do not call tools, read files or repeat completed work.'}]},deadline)
        collect()
        after=(self.workspace/'proof.txt').stat()
        self.checks.update(coldModelRemembersTokenWithoutTools=report['turnCompleted'] and report['nativeReplyMatched'] and self.commands==2,
            completedFileIdentityPreserved=all(getattr(before,k)==getattr(after,k) for k in ['st_ino','st_mtime_ns','st_size']))

    def verify(self,_root,_home,_workspace,_items,_report):
        return {**self.checks,'accepted':bool(self.checks) and all(self.checks.values())}
