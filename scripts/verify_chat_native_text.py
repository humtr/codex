"""Compare captured ChatGPT public text with one completed Native turn.

Reads existing evidence only: no model calls, Chat requests, or live state writes.
This proves available public text, not hidden reasoning, tool provenance or TUI UI.
"""
import argparse
from collections import Counter
import json
from pathlib import Path


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def source_text(message):
    content = message.get('content') or {}
    if content.get('content_type') == 'text':
        parts = content.get('parts')
        require(isinstance(parts, list) and all(isinstance(v, str) for v in parts),
                'original public text is not completely available')
        return '\n'.join(parts)
    require(content.get('content_type') == 'code' and isinstance(content.get('text'), str),
            'original public content is unsupported')
    return content['text']


def original_messages(pages, conversation, saved_result=None):
    require(bool(pages), 'original source pages are missing')
    account, branch, total = (pages[0].get(key) for key in ('account', 'branch', 'total'))
    require(all([account, branch]) and isinstance(total, int), 'original source identity is incomplete')
    messages = []
    for index, page in enumerate(pages):
        require(page.get('conversation_id') == conversation
                and page.get('account') == account and page.get('branch') == branch
                and page.get('total') == total, 'original source identity changed between pages')
        messages.extend(page['messages'])
        cursor = page['cursor']
        require(cursor.get('account') == account and cursor.get('conversation') == conversation
                and cursor.get('branch') == branch and cursor.get('count') == len(messages),
                'original pagination has a gap or overlap')
        require(page.get('complete') is (index == len(pages) - 1),
                'original pagination is incomplete or contains pages after completion')
    require(len(messages) == total and len({m['id'] for m in messages}) == total,
            'original pagination is incomplete or repeats message IDs')
    # input_cursor is the safe follow-up submission boundary, not the incoming
    # pagination cursor. Public redaction also prevents rehashing source prefixes.
    if saved_result is not None:
        binding = saved_result['source_binding']
        submitted = saved_result['input']
        suffix = saved_result['messages']
        cursor = saved_result['cursor']
        # This admission covers an exact extension of the captured branch.
        # Hidden-terminal normalization needs its own captured delivery prefix.
        require(saved_result.get('complete') is True and submitted.get('phase') == 'completed'
                and submitted.get('conversation_id') == conversation
                and binding.get('prefix_verified') is True
                and binding.get('resolution') == 'exact'
                and binding.get('requested_parent') == branch
                and binding.get('saved_parent') == branch
                and binding.get('delivery_source_cursor') == pages[-1]['cursor'],
                'saved-input result does not extend the captured complete branch')
        require(bool(suffix) and suffix[0].get('id') == submitted.get('message_id')
                and suffix[0].get('role') == 'user' and suffix[0].get('parent') == branch,
                'saved-input result starts at a different source input')
        messages.extend(suffix)
        branch = saved_result['terminal_message_id']
        require(cursor.get('account') == account and cursor.get('conversation') == conversation
                and cursor.get('branch') == branch and suffix[-1].get('id') == branch
                and cursor.get('count') == len(messages)
                and len({m['id'] for m in messages}) == len(messages)
                and saved_result.get('completed_message_id') in {m['id'] for m in suffix},
                'saved-input result cursor or message range is inconsistent')
    return branch, messages


def compare(pages, native, baseline, status, prompt, *, conversation, work, thread, saved_result=None):
    branch, messages = original_messages(pages, conversation, saved_result)
    users = [m for m in messages if m.get('role') == 'user'
             and m.get('unavailable') is None and source_text(m) == prompt]
    require(len(users) == 1, 'original input is missing or ambiguous')
    user = users[0]
    by_id = {m['id']: m for m in messages}
    ancestry, current = {}, branch
    while current != user['id']:
        require(current in by_id and current not in ancestry,
                'original branch ancestry does not reach this input')
        ancestry[current] = None
        current = by_id[current].get('parent')
    start = messages.index(user)
    following = messages[start + 1:]
    require([m['id'] for m in following] == list(reversed(ancestry)),
            'original message order differs from its branch ancestry')
    require(not any(m.get('role') == 'user' for m in following),
            'original branch has a later user input; capture the exact trial branch')
    public = [m for m in following if m.get('role') == 'assistant'
              and m.get('recipient') in (None, 'all')
              and m.get('channel') in (None, 'final', 'commentary')
              and m.get('unavailable') != 'hidden']
    require(all(m.get('unavailable') is None for m in public),
            'original public messages include unavailable content')
    finals = [m for m in public if m.get('channel') in (None, 'final')
              and m.get('end_turn') is True and m.get('status') == 'finished_successfully']
    require(len(finals) == 1, 'original final is missing, unfinished or duplicated')
    require(public[-1]['id'] == finals[0]['id'], 'original public output continues after its final')
    if saved_result is not None:
        require(saved_result['completed_message_id'] == finals[0]['id'],
                'saved-input completion differs from the original public final')
    progress = [m for m in public if m['id'] != finals[0]['id']]

    prefix = baseline['items']
    require(native['items'][:len(prefix)] == prefix, 'original Native history prefix changed')
    added = native['items'][len(prefix):]
    require(bool(added) and len({row['turn_id'] for row in added}) == 1,
            'Native evidence does not contain exactly one new turn')
    items = [row['item'] for row in added]
    native_users = [v for v in items if v.get('type') == 'userMessage']
    native_finals = [v for v in items if v.get('type') == 'agentMessage' and v.get('phase') == 'final_answer']
    native_progress = [(i, v) for i, v in enumerate(items)
                       if v.get('type') == 'agentMessage' and v.get('phase') == 'commentary']
    user_match = len(native_users) == 1 and items[0] == native_users[0] and '\n'.join(
        part['text'] for part in native_users[0]['content'] if part.get('type') == 'text') == prompt
    final_match = len(native_finals) == 1 and native_finals[0].get('text') == source_text(finals[0])
    source_counts = Counter(source_text(m) for m in progress)
    native_counts = Counter(m['text'] for _, m in native_progress)
    duplicated = [m['id'] for m in progress if native_counts[source_text(m)] > source_counts[source_text(m)]]
    missing, matched_positions, position = [], [], 0
    for message in progress:
        match = next((i for i in range(position, len(native_progress))
                      if native_progress[i][1]['text'] == source_text(message)), None)
        if match is None:
            missing.append(message['id'])
        else:
            matched_positions.append(native_progress[match][0])
            position = match + 1
    settled = (status.get('work_id') == work and status.get('thread_id') == thread
               and status.get('phase') == 'completed' and status.get('pending_calls') == []
               and status.get('native_status', {}).get('type') in ('idle', 'notLoaded'))
    progress_match = not missing and not duplicated
    ordered = len(native_finals) == 1 and all(
        i < items.index(native_finals[0]) for i in matched_positions)
    return {'accepted_public_text': user_match and final_match and progress_match and ordered and settled,
            'scope': 'available original public text; not tool provenance or live TUI',
            'conversation_id': conversation, 'branch': branch, 'work_id': work, 'thread_id': thread,
            'source_user_id': user['id'], 'source_final_id': finals[0]['id'],
            'source_pages': len(pages), 'saved_input_result': saved_result is not None,
            'source_messages': len(messages),
            'native_prefix_items': len(prefix), 'native_new_items': len(added),
            'input_matches': user_match, 'final_matches': final_match, 'native_settled': settled,
            'public_order_matches': ordered,
            'public_progress_matches': progress_match, 'source_progress_count': len(progress),
            'native_progress_count': len(native_progress), 'missing_progress_ids': missing,
            'duplicated_progress_ids': duplicated,
            'source_unavailable_messages': sum(m.get('unavailable') is not None for m in following)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-page', type=Path, action='append', required=True)
    parser.add_argument('--saved-input-result', type=Path)
    for name in ['native-history', 'baseline-history', 'native-status', 'prompt', 'report']:
        parser.add_argument('--' + name, type=Path, required=True)
    for name in ['conversation', 'work', 'thread']:
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    def load(path):
        return json.loads(path.read_text())
    try:
        with args.prompt.open(encoding='utf-8', newline='') as stream:
            prompt = stream.read()
        result = compare([load(p) for p in args.source_page], load(args.native_history),
                         load(args.baseline_history), load(args.native_status), prompt,
                         conversation=args.conversation, work=args.work, thread=args.thread,
                         saved_result=load(args.saved_input_result) if args.saved_input_result else None)
    except (ValueError, KeyError, TypeError) as error:
        result = {'accepted_public_text': False, 'error': str(error)}
    with args.report.open('x') as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2)
        stream.write('\n')
    print(json.dumps(result, ensure_ascii=False))
    return 0 if result['accepted_public_text'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
