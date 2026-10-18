// App-owned capabilities only. Rust owns native identity, catalog, history,
// batching, result correlation, progress projection and lifecycle decisions.
async function termuxCodexBridge(command) {
  const slot = Symbol.for('termux.codex.app.bridge.v1');
  const fail = code => { throw Error(code); };
  let state = globalThis[slot];
  if (command.op === 'init') {
    const mode=command.returnPath??'local_function';
    if (!['local_function','mcp'].includes(mode)) fail('app_return_path_invalid');
    if (state) return state.owner === command.owner && state.mode===mode ? {ready: true} : fail('app_bridge_owned');
    let node = document.querySelector('[contenteditable=true]') || document.querySelector('button');
    while (node && !Object.keys(node).some(k => k.startsWith('__reactFiber'))) node = node.parentElement;
    let fiber = node?.[Object.keys(node).find(k => k.startsWith('__reactFiber'))], scope;
    for (let i = 0; fiber && i < 400 && !scope; i++, fiber = fiber.return) {
      for (let hook = fiber.memoizedState, j = 0; hook && j < 300; hook = hook.next, j++) {
        const value = hook.memoizedState?.current;
        if (typeof value?.get === 'function' && typeof value?.set === 'function' && value.node && value.chain && value.query) { scope = value; break; }
      }
    }
    if (!scope) return {ready: false};
    const api = await import(command.bindings.initial);
    const shared = await import(command.bindings.shared);
    const auth=scope.get(shared.xNt);
    if(!auth||auth.authLoading||auth.accountLoading)return {ready:false};
    if (!['aCn','yRt','eHt'].every(k => typeof api[k] === 'function') || !api.JRn?.kind) fail('app_local_functions_unavailable');
    if (mode==='local_function') {
      if (typeof api.gRt!=='function'||typeof api.bRt!=='function') fail('app_local_functions_unavailable');
      api.gRt();
      if (!scope.get(shared.HAt)) fail('app_local_functions_unavailable');
    }
    const client = scope.get(api.oMn), wire = shared.$wt;
    if (!['prepareCompletionStream','startCompletionStream'].every(k => typeof client?.[k] === 'function') || typeof wire?.streamPost !== 'function') fail('app_stream_unavailable');
    state = {owner: command.owner, mode, scope, api, shared, client, wire, lanes: new Map(), saved: {}, hooks: {}};
    const owner = request => [...state.lanes.values()].find(l =>
      request?.conversation_id != null && [l.localId,l.serverId].includes(request.conversation_id) ||
      request?.local_function_signatures?.some(f => f.name === l.signature.name) ||
      request?.messages?.some(m => m.metadata?.termux_codex_lane === l.marker));
    for (const name of ['prepareCompletionStream','startCompletionStream']) {
      state.saved[name] = client[name];
      state.hooks[name] = function(...args) {
        const request = name === 'startCompletionStream' ? args[0]?.request : args[0], lane = owner(request);
        if (!lane) return state.saved[name].apply(this,args);
        if (lane.project && request.gizmo_id !== lane.project) fail('project_membership_unconfirmed');
        const template = request.local_function_signatures?.some(f => f.name === 'handoff');
        const prepared = lane.declared && request.local_function_signatures?.length === 1 && JSON.stringify(request.local_function_signatures[0]) === JSON.stringify(lane.signature);
        if (mode==='local_function' && (!scope.get(shared.HAt) || !template && !prepared)) fail('app_local_declaration_unavailable');
        if (template) lane.declared = true;
        const owned = {...request, local_function_signatures: mode==='mcp'?[]:[lane.signature],...(mode==='mcp'?{local_function_names:[]}:{})};
        if (name === 'prepareCompletionStream') return state.saved[name].call(this,owned,args[1]);
        const options = {...args[0],request: owned}, epoch = lane.epoch;
        lane.streamActive = true;
        for (const event of ['onDecodedEvent','onUpdate']) {
          const previous = options[event];
          options[event] = update => {
            let message = update?.update?.message ?? update?.message;
            if (message?.message) message = message.message;
            if (lane.epoch === epoch && message?.id && message.author && !lane.older.has(message.id)) lane.messages.set(message.id,message);
            return previous?.(update);
          };
        }
        const previousDone = options.onComplete, previousError = options.onError;
        options.onComplete = value => { if (lane.epoch === epoch) { lane.terminals.push(value.reason); lane.streamActive = false; } return previousDone?.(value); };
        options.onError = value => { if (lane.epoch === epoch) lane.error = 'app_stream_failed'; return previousError?.(value); };
        return state.saved[name].call(this,options);
      };
      client[name] = state.hooks[name];
    }
    state.saved.streamPost = wire.streamPost;
    state.hooks.streamPost = function(path,options) {
      if (path !== '/f/conversation') return state.saved.streamPost.call(this,path,options);
      let request; try { request = JSON.parse(options?.body); } catch {}
      const lane = owner(request);
      if (!lane) return state.saved.streamPost.call(this,path,options);
      const ids = request.messages?.map(m => m.id).join(',');
      if (!ids || lane.sent.has(ids) || lane.roundPrepared) fail('app_dispatch_replayed');
      lane.roundPrepared = true;
      const roles = request.messages.map(m => m.author?.role);
      if (mode==='mcp' && (request.local_function_signatures?.length || request.local_function_names?.length || !roles.includes('user') || roles.includes('tool'))) fail('app_dispatch_invalid');
      if (roles.some(r => !['user','tool','system','assistant'].includes(r)) || !roles.includes('user') && !roles.includes('tool') || lane.resultOnly && roles.includes('user')) fail('app_dispatch_invalid');
      lane.sent.add(ids);
      const before = options.chatgptSse?.onBeforeRequestStart;
      return state.saved.streamPost.call(this,path,{...options,retry: undefined,chatgptSse: {...options.chatgptSse,onBeforeRequestStart(value) {
        if (lane.starts.has(ids)) fail('app_dispatch_replayed');
        before?.(value);
        lane.starts.add(ids); lane.roundStarts++;
        lane.userMessages += roles.filter(r => r === 'user').length;
        lane.toolMessages += roles.filter(r => r === 'tool').length;
      }}});
    };
    wire.streamPost = state.hooks.streamPost;
    globalThis[slot] = state;
    return {ready: true};
  }
  if (!state || state.owner !== command.owner) fail('app_bridge_owner_mismatch');
  const identity = () => {
    const auth=state.scope.get(state.shared.xNt);
    if(auth?.authLoading||auth?.accountLoading)fail('app_auth_loading');
    if (!auth || auth.authLoading || auth.accountLoading || auth.authMethod!=='chatgpt' ||
        typeof auth.accountId!=='string' || !auth.accountId || auth.accountId!==auth.authenticatedAccountId ||
        typeof auth.userId!=='string' || !auth.userId || auth.userId!==auth.authenticatedUserId ||
        !['personal','workspace'].includes(auth.accountStructure)) fail('app_auth_context_unavailable');
    return {account:auth.accountId,user:auth.userId,structure:auth.accountStructure};
  };
  if (['project_list','project_read','project_create'].includes(command.op)) {
    const check = () => { const current=identity();if (['account','user','structure'].some(k=>current[k]!==command.identity?.[k])) fail('app_account_changed'); };
    const api=state.wire;
    if (typeof api.safeGet!=='function'||typeof api.safePost!=='function') fail('app_project_operations_unavailable');
    const project = value => {
      const g=value?.gizmo,auth=state.scope.get(state.shared.xNt);
      if(typeof g?.id!=='string'||typeof g.display?.name!=='string')fail('invalid_project_identity');
      const user=auth.accountStructure==='personal'?auth.accountUserId?.split('__')[0]:auth.accountUserId;
      return {id:g.id,name:g.display.name,compatible:g.id.startsWith('g-p-')&&g.author?.user_id===user&&
        !g.first_party_project_type&&!g.gizmo_snorlax_type&&g.share_recipient==='private'&&
        value.hasSharingSubjects!==true&&(g.sharing?.subjects?.length??0)===0};
    };
    check();
    if(command.op==='project_read') {
      const value=await api.safeGet('/gizmos/{gizmo_id_or_short_url}',{parameters:{path:{gizmo_id_or_short_url:command.project},query:{include_file_limits:true}}});
      check();return project(value);
    }
    if(command.op==='project_create') {
      let value;
      try { value=await api.safePost('/projects',{requestBody:{name:command.name,instructions:''},retry:'never',assertRequestCurrent:check}); }
      catch(error) {
        if([400,401,403,404,422,429].includes(error.responseStatus))fail('project_creation_rejected');
        throw error;
      }
      check();if(value.error!=null&&!value.resource)fail('project_creation_rejected');
      if(!value.resource)fail('project_creation_unconfirmed');return project(value.resource);
    }
    const result=[],seen=new Set();let cursor=null;
    do {
      const value=await api.safeGet('/gizmos/snorlax/sidebar',{parameters:{query:{conversations_per_gizmo:0,cursor,limit:20,owned_only:true}}});
      check();if(!Array.isArray(value.items))fail('invalid_project_list');
      for(const item of value.items)result.push(project(item.gizmo));
      cursor=value.cursor??null;
      if(cursor!==null&&(typeof cursor!=='string'||seen.has(cursor))||result.length>5000)fail('project_list_bound_exceeded');
      seen.add(cursor);
    }while(cursor!==null);
    return result;
  }
  if (command.op === 'dispose') {
    if (state.lanes.size) fail('app_lanes_active');
    for (const name of ['prepareCompletionStream','startCompletionStream']) if (state.client[name] === state.hooks[name]) state.client[name] = state.saved[name];
    if (state.wire.streamPost === state.hooks.streamPost) state.wire.streamPost = state.saved.streamPost;
    delete globalThis[slot];
    return {disposed:true};
  }
  if (command.op === 'identity') {
    // Return identifiers only; never tokens, email or the complete auth state.
    return {...identity(),present:state.lanes.has(command.lane)};
  }
  if (command.op === 'open') {
    const existing=state.lanes.get(command.lane);
    if (existing) {
      if (JSON.stringify(existing.signature)!==JSON.stringify(command.signature) || existing.project!==(command.project??null)) fail('app_lane_declaration_changed');
      return {localId:existing.localId};
    }
    // A released final lane restores app methods; retain the owned session so
    // a later lane can reattach without replacing the authenticated app.
    for (const name of ['prepareCompletionStream','startCompletionStream']) {
      if (state.client[name] !== state.saved[name] && state.client[name] !== state.hooks[name]) fail('app_hooks_changed');
      state.client[name] = state.hooks[name];
    }
    if (state.wire.streamPost !== state.saved.streamPost && state.wire.streamPost !== state.hooks.streamPost) fail('app_hooks_changed');
    state.wire.streamPost = state.hooks.streamPost;
    const localId = state.api.aCn(state.scope);
    state.lanes.set(command.lane,{localId,marker:command.lane,project:command.project??null,signature: command.signature,messages: new Map(),sent: new Set(),starts: new Set(),older: new Set(),epoch: 0,userMessages: 0,toolMessages: 0,terminals: []});
    return {localId};
  }
  const lane = state.lanes.get(command.lane);
  if (!lane) {
    if (command.op === 'release') return {released:true};
    fail('app_lane_missing');
  }
  const snapshot = () => {
    const hostResponses = new Set();
    for (const node of Object.values(state.scope.get(state.api.JRn,lane.localId) || {})) {
      const message = node.message;
      const metadata=message?.metadata,plugin=metadata?.jit_plugin_data??metadata?.elicitation_metadata?.plugin_data;
      const target=plugin?.from_client?.target_message_id??metadata?.codex_mcp_elicitation_response?.target_message_id;
      if(typeof target==='string'&&target.length>0)hostResponses.add(target);
      if (message?.id && (message.author?.role === 'assistant' || state.mode==='mcp'&&message.author?.role==='tool') && !lane.older.has(message.id)) lane.messages.set(message.id,message);
    }
    // Public messages and our own tool calls only; never export hidden reasoning.
    const messages = [...lane.messages.values()].filter(m => state.mode==='mcp'&&m.author?.role==='tool' || m.author?.role === 'assistant' &&
      (m.recipient === 'local.' + lane.signature.name || ((m.recipient == null || m.recipient === 'all') &&
       [undefined,null,'commentary','final'].includes(m.channel) && !m.metadata?.is_visually_hidden_from_conversation)));
    return {round:lane.round,messages,hostResponses:[...hostResponses],accepted: lane.accepted,error: lane.error,terminals: lane.terminals,roundStarts: lane.roundStarts,
      dispatches: lane.starts.size,userMessages: lane.userMessages,toolMessages: lane.toolMessages,serverId: lane.serverId,
      stopped: lane.stopRequested === true && lane.streamActive === false && lane.terminals.some(reason => ['done','abort','aborted','cancelled','canceled'].includes(reason))};
  };
  if (command.op === 'snapshot') return snapshot();
  if (command.op === 'project_membership') {
    if (!lane.project || !lane.serverId || lane.pending || lane.streamActive) fail('project_membership_unsettled');
    const before=identity();
    const conversation=await state.wire.safeGet('/conversation/{conversation_id}',{parameters:{path:{conversation_id:lane.serverId}}});
    if (JSON.stringify(identity())!==JSON.stringify(before)) fail('app_account_changed');
    if (conversation.gizmo_id!==lane.project) fail('project_membership_unconfirmed');
    return {verified:true};
  }
  if (command.op === 'stop') {
    lane.stopRequested = true;
    lane.controller?.abort();
    await state.api.eHt(state.scope,lane.localId);
    // Completion/cancellation observation is returned, not guessed from HTTP.
    return snapshot();
  }
  if (command.op === 'submit') {
    if (state.mode==='mcp' && command.results?.length) fail('app_mcp_result_resubmit_forbidden');
    if (lane.round === command.round) return {started:true,reused:true};
    if (lane.pending) fail('app_lane_busy');
    lane.round = command.round;
    lane.epoch++; lane.messages.clear(); lane.terminals = []; lane.error = null; lane.accepted = false; lane.stopRequested = false; lane.roundStarts = 0; lane.roundPrepared = false;
    lane.older = new Set(Object.values(state.scope.get(state.api.JRn,lane.localId) || {}).map(n => n.message?.id).filter(Boolean));
    lane.controller = new AbortController();
    const message = ({role,text,hidden}) => ({id: crypto.randomUUID(),author: {role: role === 'developer' ? 'system' : role},content: {content_type: 'text',parts: [text]},metadata: {is_visually_hidden_from_conversation: hidden || role === 'developer',termux_codex_lane:command.lane},recipient: 'all',status: 'finished_successfully',end_turn: role === 'assistant' ? true : null});
    const results = command.results || [], context = (command.messages || []).map(message);
    const instructions = command.instructions || [];
    lane.resultOnly = results.length > 0 && !context.some(m => m.author.role === 'user');
    const options = {conversationId: lane.localId,projectId:lane.project??undefined,model: command.model,thinkingEffort: command.thinkingEffort ?? null,isTemporaryChat: false,
      onServerThreadIdChange: id => { lane.serverId = id; },requireResponseAcceptance: true,startupSignal: lane.controller.signal};
    let pending;
    if (results.length === 1 && !context.length && !instructions.length) {
      pending = state.api.bRt(state.scope,{...options,callId: results[0].id,toolName: lane.signature.name,result: results[0].result});
    } else {
      const imported = results.map(r => ({id: crypto.randomUUID(),author: {role:'tool',metadata:{},name:lane.signature.name},channel:'commentary',content:{content_type:'code',text:JSON.stringify({call_id:r.id,result:r.result,tool:lane.signature.name})},metadata:{is_visually_hidden_from_conversation:true},recipient:'all',status:'finished_successfully',end_turn:null,weight:1,create_time:null,update_time:null}));
      imported.push(...context);
      if (!imported.length) fail('app_messages_required');
      for (const item of imported) lane.older.add(item.id);
      pending = state.api.yRt(state.scope,{...options,prompt:'',prependMessages:imported.slice(0,-1),
        ...(results.length ? {daybreakContinuationMessageId:results[0].id} : {}),
        userCompletionMessages:{message:imported.at(-1),extraDeveloperInstructionMessages:instructions.map(text => message({role:'developer',text,hidden:true}))}});
    }
    lane.pending = true;
    const epoch = lane.epoch;
    Promise.resolve(pending).then(value => {
      if (epoch !== lane.epoch) return;
      lane.accepted = !!value; lane.pending = false;
    },() => { if (epoch === lane.epoch) { lane.error = 'app_submit_failed'; lane.pending = false; } });
    return {started: true};
  }
  if (command.op === 'release') {
    if (lane.pending) fail('app_lane_busy');
    state.lanes.delete(command.lane);
    if (!state.lanes.size) {
      for (const name of ['prepareCompletionStream','startCompletionStream']) if (state.client[name] === state.hooks[name]) state.client[name] = state.saved[name];
      if (state.wire.streamPost === state.hooks.streamPost) state.wire.streamPost = state.saved.streamPost;
      // Keep only the owner and reusable app capabilities; no lane remains.
    }
    return {released:true};
  }
  fail('app_operation_unknown');
}
