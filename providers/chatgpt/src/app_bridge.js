// App-owned capabilities only. Rust owns native identity, catalog, history,
// batching, result correlation, progress projection and lifecycle decisions.
async function termuxCodexBridge(command) {
  const slot = Symbol.for('termux.codex.app.bridge.v1');
  const fail = code => { throw Error(code); };
  const failureCode=(lane,fallback)=>['assignment_app_budget_exhausted','assignment_time_budget_exhausted'].includes(lane.admissionCause)?lane.admissionCause:fallback;
  const completion=lane=>{
    const epoch=lane.epoch,serial=(lane.streamSerial??0)+1;
    return status=>{
      if(lane.epoch!==epoch||lane.stopRequested||(lane.streamSerial??0)>serial)return;
      // SDK polling recovery can finish without a raw stream terminal. A later
      // host-reply stream owns its own completion, even within the same epoch.
      if(status==='completed'){lane.terminals=['done'];lane.streamActive=false;}
      else if(['failed','interrupted'].includes(status)){
        lane.error=status==='failed'?'app_completion_failed':'app_completion_interrupted';
        lane.streamActive=false;
      }
    };
  };
  // The app may address an approval to the invoking assistant message rather
  // than the tool's confirmation message. Admit only targets offered by that
  // observed confirmation, never an arbitrary ancestor or known message.
  const hostTargets = message => {
    const metadata=message?.metadata,plugin=metadata?.jit_plugin_data??metadata?.elicitation_metadata?.plugin_data;
    if(message?.author?.role!=='tool'||!(['confirm_action','approve_tool_in_context_scope','plugin_auth_required','oauth_required','link_required'].includes(plugin?.from_server?.type)||metadata?.codex_plugin_auth_required?.status==='pending'))return [];
    const body=plugin?.from_server?.body;
    return [message.id,body?.target_message_id,body?.codex_mcp_elicitation?.target_message_id,...(body?.actions??[]).flatMap(action=>['allow','allow_once','always_allow','deny','link_created','oauth_redirect'].map(key=>action[key]?.target_message_id).concat((action.split_action_options??[]).map(o=>o.action?.target_message_id)))].filter(v=>typeof v==='string'&&v.length>0);
  };
  const appScope=()=>{
    let node=document.querySelector('[contenteditable=true]')||document.querySelector('button');
    while(node&&!Object.keys(node).some(k=>k.startsWith('__reactFiber')))node=node.parentElement;
    let fiber=node?.[Object.keys(node).find(k=>k.startsWith('__reactFiber'))],scope;
    for(let i=0;fiber&&i<400&&!scope;i++,fiber=fiber.return)for(let hook=fiber.memoizedState,j=0;hook&&j<300;hook=hook.next,j++){
      const value=hook.memoizedState?.current;
      if(typeof value?.get==='function'&&typeof value?.set==='function'&&value.node&&value.chain&&value.query){scope=value;break;}
    }
    return scope??null;
  };
  const appContext=async()=>{
    const scope=appScope();if(!scope)return null;
    let api,shared;
    try{api=await import(command.bindings.initial);shared=await import(command.bindings.shared);}catch{fail('app_bindings_unavailable');}
    return {scope,api,shared,client:scope.get(api.oMn),wire:shared.$wt};
  };
  let state = globalThis[slot];
  const observation=command.readOnlyContext===true;
  if(observation){
    if(!['identity','host_status','model_catalog','chat_list','chat_read','chat_page','project_list','project_read'].includes(command.op))fail('app_operation_not_read_only');
    const context=await appContext();if(!context)fail('app_scope_unavailable');
    const owned=state?.owner===command.owner;
    state={...context,owner:command.owner,revision:globalThis[slot]?.revision??null,lanes:owned?state.lanes:new Map(),observation:true};
  }
  if (command.op === 'init') {
    const mode=command.returnPath??'local_function';
    if (!['local_function','mcp'].includes(mode)) fail('app_return_path_invalid');
    if(typeof command.bridgeRevision!=='string'||!/^[0-9a-f]{64}$/.test(command.bridgeRevision))fail('app_bridge_revision_required');
    if (state) {
      if(state.owner!==command.owner)fail('app_bridge_owned');
      if(state.revision!==command.bridgeRevision)fail('app_bridge_revision_conflict');
    }
    const context=await appContext();if(!context)return {ready:false};
    const {scope,api,shared,client,wire}=context;
    const auth=scope.get(shared.xNt);
    if(!auth||auth.authLoading||auth.accountLoading)return {ready:false};
    if(state&&state.scope===scope&&state.client===client&&state.wire===wire)return {ready:true};
    if (!['aCn','yRt','eHt'].every(k => typeof api[k] === 'function') || !api.JRn?.kind) fail('app_local_functions_unavailable');
    if (mode==='local_function') {
      if (typeof api.gRt!=='function') fail('app_local_functions_unavailable');
      api.gRt();
      if (!scope.get(shared.HAt)) fail('app_local_functions_unavailable');
    }
    if (!['prepareCompletionStream','startCompletionStream'].every(k => typeof client?.[k] === 'function') || typeof wire?.streamPost !== 'function') fail('app_stream_unavailable');
    if(state){
      if([...state.lanes.values()].some(l=>l.pending||l.approvalPending||l.streamActive))fail('app_lanes_active');
      // Login can replace the React scope without replacing the renderer.
      // Retire only our passive hooks; saved chats and native journals remain.
      for(const name of ['prepareCompletionStream','startCompletionStream'])if(state.client[name]===state.hooks[name])state.client[name]=state.saved[name];
      if(state.wire.streamPost===state.hooks.streamPost)state.wire.streamPost=state.saved.streamPost;
    }
    state = {owner: command.owner, revision:command.bridgeRevision, mode, scope, api, shared, client, wire, lanes: new Map(), workBudgets:new Map(), saved: {}, hooks: {}};
    const owner = request => [...state.lanes.values()].find(l => {
      const messages=request?.messages??[],ids=messages.map(m=>m.id).join(',');
      if(request?.local_function_signatures?.some(f=>f.name===l.signature.name)
        || messages.some(m=>m.metadata?.termux_codex_lane===l.marker||m.author?.name===l.signature.name)
        || ids&&l.sent.has(ids))return true;
      if(request?.conversation_id==null||![l.localId,l.serverId,l.uiLocalId].includes(request.conversation_id))return false;
      return l.pending||l.streamActive||l.approvalPending||messages.some(m=>{
        const meta=m.metadata,p=meta?.jit_plugin_data??meta?.elicitation_metadata?.plugin_data;
        const target=p?.from_client?.target_message_id??meta?.codex_mcp_elicitation_response?.target_message_id;
        return m.author?.role==='tool'&&[...l.messages.values()].some(prompt=>hostTargets(prompt).includes(target));
      });
    });
    for (const name of ['prepareCompletionStream','startCompletionStream']) {
      state.saved[name] = client[name];
      state.hooks[name] = function(...args) {
        const request = name === 'startCompletionStream' ? args[0]?.request : args[0], lane = owner(request);
        if (!lane) return state.saved[name].apply(this,args);
        const mode=lane.mode;
        if(name==='startCompletionStream'&&mode==='mcp'&&request?.messages?.some(m=>{
          const meta=m.metadata,p=meta?.jit_plugin_data??meta?.elicitation_metadata?.plugin_data;
          const target=p?.from_client?.target_message_id??meta?.codex_mcp_elicitation_response?.target_message_id;
          return m.author?.role==='tool'&&[...lane.messages.values()].some(prompt=>hostTargets(prompt).includes(target));
        })) {
          lane.approvalPending=false;
          if(lane.stopRequested)fail('app_dispatch_stopped');
        }
        if(lane.stopRequested)fail('app_dispatch_stopped');
        if (lane.project && request.gizmo_id !== lane.project) fail('project_membership_unconfirmed');
        const template = request.local_function_signatures?.some(f => f.name === 'handoff');
        const prepared = lane.declared && request.local_function_signatures?.length === 1 && JSON.stringify(request.local_function_signatures[0]) === JSON.stringify(lane.signature);
        if (mode==='local_function' && (!scope.get(shared.HAt) || !template && !prepared)) fail('app_local_declaration_unavailable');
        if (template) lane.declared = true;
        const owned = {...request, local_function_signatures: mode==='mcp'?[]:[lane.signature],...(mode==='mcp'?{local_function_names:[]}:{})};
        if (name === 'prepareCompletionStream') return state.saved[name].call(this,owned,args[1]);
        const options = {...args[0],request: owned}, epoch = lane.epoch;
        const serial=lane.streamSerial=(lane.streamSerial??0)+1;
        lane.streamActive = true;
        const observeError=(stage,error,network)=>{
          if(lane.epoch!==epoch||lane.streamSerial!==serial)return;
          const value={stage,http_status:Number.isInteger(error?.responseStatus)?error.responseStatus:null,
            network:typeof network==='boolean'?network:error?.errorKind==='network'?true:null};
          lane.transportObservation??={};
          lane.transportObservation.first_error??=value;
        };
        const previousObserved=options.onStreamErrorObserved,previousResume=options.onResume;
        options.onStreamErrorObserved=value=>{observeError('stream',value);return previousObserved?.(value);};
        options.onResume=value=>{
          if(lane.epoch===epoch&&lane.streamSerial===serial&&['attempted','started','succeeded','error','response'].includes(value?.type)){
            lane.transportObservation??={};
            lane.transportObservation.resume={stage:value.type,attempt:Number.isSafeInteger(value.attemptCount)?value.attemptCount:null};
            if(value.type==='error')observeError('resume',value.error,value.isNetworkError);
          }
          return previousResume?.(value);
        };
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
        options.onComplete = value => { if (lane.epoch === epoch&&lane.streamSerial===serial) { lane.terminals.push(value.reason); lane.streamActive = false; } return previousDone?.(value); };
        options.onError = value => { if (lane.epoch === epoch&&lane.streamSerial===serial) {lane.error = failureCode(lane,'app_stream_failed');lane.streamActive=false;} return previousError?.(value); };
        const failed=error=>{if(lane.epoch===epoch&&lane.streamSerial===serial){lane.error=failureCode(lane,'app_stream_failed');lane.streamActive=false;}throw error;};
        try {const started=state.saved[name].call(this,options);return started&&typeof started.then==='function'?Promise.resolve(started).catch(failed):started;}
        catch(error){return failed(error);}
      };
      client[name] = state.hooks[name];
    }
    state.saved.streamPost = wire.streamPost;
    state.hooks.streamPost = function(path,options) {
      if (path !== '/f/conversation') return state.saved.streamPost.call(this,path,options);
      let request; try { request = JSON.parse(options?.body); } catch {}
      const lane = owner(request);
      if (!lane) return state.saved.streamPost.call(this,path,options);
      const mode=lane.mode;
      const ids = request.messages?.map(m => m.id).join(',');
      const roles = request.messages?.map(m => m.author?.role) || [];
      const rejectDispatch=code=>{lane.rejectedBeforeDispatch=true;lane.admissionCause=code;if(!lane.priorStreamUnsettled)lane.streamActive=false;fail(code);};
      if(lane.stopRequested)rejectDispatch('app_dispatch_stopped');
      const auth=appScope()?.get(state.shared.xNt);
      if(!auth||auth.authLoading||auth.accountLoading||auth.authMethod!=='chatgpt'||auth.accountId!==auth.authenticatedAccountId||auth.userId!==auth.authenticatedUserId||auth.accountId!==lane.accountIdentity.account||auth.userId!==lane.accountIdentity.user||auth.accountStructure!==lane.accountIdentity.structure)rejectDispatch('app_account_changed');
      const bootstrap=lane.nonUserBootstrap===true && (lane.signature.name==='operator_no_local_tools'||/^operator_local_[0-9a-f]{48}$/.test(lane.signature.name)) && !lane.priorStreamUnsettled && roles.length>0 && roles.every(role=>role==='system');
      const reply=request.messages?.length===1?request.messages[0]:null;
      const metadata=reply?.metadata,plugin=metadata?.jit_plugin_data??metadata?.elicitation_metadata?.plugin_data;
      const target=plugin?.from_client?.target_message_id??metadata?.codex_mcp_elicitation_response?.target_message_id;
      const prompts=typeof target==='string'?[...lane.messages.values()].filter(m=>hostTargets(m).includes(target)):[];
      const hostReply=mode==='mcp'&&!lane.stopRequested&&reply?.author?.role==='tool'&&prompts.length===1;
      if (!ids || lane.sent.has(ids) || lane.roundPrepared&&!hostReply || hostReply&&hostTargets(prompts[0]).some(t=>lane.hostReplies.has(t))) {
        lane.rejectedDispatch={messageCount:request.messages?.length??0,roles,duplicateIds:lane.sent.has(ids),roundPrepared:lane.roundPrepared===true,hostReply,replySeen:lane.hostReplies.has(target),stopped:lane.stopRequested===true,hasClientReply:!!plugin?.from_client,hasCodexReply:!!metadata?.codex_mcp_elicitation_response,matchingConfirmations:prompts.length};
        fail('app_dispatch_replayed');
      }
      if (!hostReply) lane.roundPrepared = true;
      if (mode==='mcp' && (request.local_function_signatures?.length || request.local_function_names?.length || !hostReply&&!bootstrap&&(!roles.includes('user') || roles.includes('tool')))) rejectDispatch('app_dispatch_invalid');
      if (roles.some(r => !['user','tool','system','assistant'].includes(r)) || !bootstrap&&!roles.includes('user') && !roles.includes('tool') || lane.resultOnly && roles.includes('user')) rejectDispatch('app_dispatch_invalid');
      if(lane.budget) {
        if(lane.budget.reserved>=lane.budget.maxDispatches)rejectDispatch('assignment_app_budget_exhausted');
        if(lane.budget.deadlineUnixMs!=null&&Date.now()>=lane.budget.deadlineUnixMs)rejectDispatch('assignment_time_budget_exhausted');
        // Reserve before handing control to an uncertain network operation.
        lane.budget.reserved++;
      }
      lane.sent.add(ids);
      const before = options.chatgptSse?.onBeforeRequestStart;
      return state.saved.streamPost.call(this,path,{...options,retry: undefined,chatgptSse: {...options.chatgptSse,onBeforeRequestStart(value) {
        if (lane.starts.has(ids)) fail('app_dispatch_replayed');
        before?.(value);
        lane.starts.add(ids); lane.roundStarts++;
        if(!hostReply&&roles.includes('user'))lane.dispatchedInput={
          conversation_id:request.conversation_id??null,parent_message_id:request.parent_message_id??null,
          requested_model:request.model??null,
          users:request.messages.filter(m=>m.author?.role==='user').map(m=>{
            const prepared=lane.preparedUsers?.get(m.id);
            return {id:m.id,fingerprint:prepared?.content===JSON.stringify(m.content)?prepared.fingerprint??null:null,
              hidden:m.metadata?.is_visually_hidden_from_conversation===true};
          })
        };
        if(hostReply){lane.hostReplies.add(target);lane.hostReplyMessages++;lane.roundHostReplies++;lane.terminals=[];lane.error=null;lane.rejectedBeforeDispatch=false;}
        lane.userMessages += roles.filter(r => r === 'user').length;
        if(!hostReply)lane.toolMessages += roles.filter(r => r === 'tool').length;
      }}});
    };
    wire.streamPost = state.hooks.streamPost;
    globalThis[slot] = state;
    return {ready: true};
  }
  if (!state && command.op === 'release') return {released:true};
  if (!state || state.owner !== command.owner) fail('app_bridge_owner_mismatch');
  if(!observation&&command.bridgeRevision!=null&&state.revision!==command.bridgeRevision)fail('app_bridge_revision_conflict');
  const identity = (scope=state.scope) => {
    const auth=scope.get(state.shared.xNt);
    if(auth?.authLoading||auth?.accountLoading)fail('app_auth_loading');
    if (!auth || auth.authLoading || auth.accountLoading || auth.authMethod!=='chatgpt' ||
        typeof auth.accountId!=='string' || !auth.accountId || auth.accountId!==auth.authenticatedAccountId ||
        typeof auth.userId!=='string' || !auth.userId || auth.userId!==auth.authenticatedUserId ||
        !['personal','workspace'].includes(auth.accountStructure)) fail('app_auth_context_unavailable');
    return {account:auth.accountId,user:auth.userId,structure:auth.accountStructure};
  };
  // Retrieval pacing is shared transport metadata, separate from lane ownership.
  // It survives namespace disposal so a new reader cannot reset an account's429.
  const readPolicy=()=>{
    const key=JSON.stringify(identity()),slot=Symbol.for('termux.codex.app.read-backoff.v1');
    const entries=globalThis[slot]??=new Map();
    if(!entries.has(key)){
      if(entries.size>=8)entries.delete(entries.keys().next().value);
      entries.set(key,{failures:0,notBefore:0,lastFailure:null,inFlight:false,requests:0,successes:0,rateLimits:0,cooldownRejections:0,pendingRejections:0});
    }
    const policy=entries.get(key);
    // Read pacing can outlive an earlier bridge revision in this renderer.
    for(const name of ['requests','successes','rateLimits','cooldownRejections','pendingRejections'])policy[name]??=0;
    return policy;
  };
  const retryStatus=()=>{
    const p=readPolicy();
    return {cooling_down:p.notBefore>Date.now(),retry_after_seconds:Math.max(0,Math.ceil((p.notBefore-Date.now())/1000)),last_failure:p.lastFailure,
      requests:{scope:'renderer_account_lifetime',wire_requests:p.requests,successful_replies:p.successes,http429_replies:p.rateLimits,cooldown_rejections:p.cooldownRejections,in_flight_rejections:p.pendingRejections}};
  };
  const readSavedChat=async(path,parameters,before)=>{
    if(typeof state.wire.safeGet!=='function')fail('app_chat_operations_unavailable');
    const policy=readPolicy();
    if(policy.notBefore>Date.now()){policy.cooldownRejections++;fail(policy.lastFailure.code);}
    if(policy.inFlight){policy.pendingRejections++;fail('app_chat_read_pending');}
    const options={retry:'never',additionalHeaders:{'ChatGPT-Account-ID':before.account},assertRequestCurrent:()=>{if(JSON.stringify(identity())!==JSON.stringify(before))fail('app_account_changed');}};
    let result;
    policy.inFlight=true;
    try {
      policy.requests++;
      result=await state.wire.safeGet(path,{...options,parameters});
      policy.successes++;
      options.assertRequestCurrent();
    } catch(error) {
      options.assertRequestCurrent();
      if(error?.message==='app_chat_response_invalid')throw error;
      if(error?.responseStatus===401)fail('app_chat_auth_expired');
      if(error?.responseStatus===403)fail('app_chat_forbidden');
      if(error?.responseStatus===404)fail('app_chat_not_found');
      const status=Number.isInteger(error?.responseStatus)?error.responseStatus:null;
      if(status===429)policy.rateLimits++;
      const code=status===429?'app_chat_rate_limited':status!==null&&status>=500&&status<=599?'app_chat_temporarily_unavailable':'app_chat_request_failed';
      const now=Date.now();policy.failures=policy.lastFailure?.code===code?policy.failures+1:1;
      const headers=error?.responseHeaders??error?.headers;
      const header=typeof headers?.get==='function'?headers.get('retry-after'):headers?.['retry-after'];
      const parsed=header==null?0:Number(header),retry=Number.isFinite(parsed)?Math.max(0,parsed):Math.max(0,(Date.parse(header)-now)/1000);
      const base=status===429?60:status==null?2:5,cap=status===429?300:60;
      const seconds=Math.max(Math.min(cap,base*2**Math.min(10,policy.failures-1)),Number.isFinite(retry)?retry:0);
      policy.notBefore=now+Math.ceil(seconds*1000);
      policy.lastFailure={code,http_status:status,observed_at_unix_ms:now};
      const writer=globalThis[slot];
      if(writer?.owner===command.owner)writer.lastReadFailure=policy.lastFailure;
      fail(code);
    } finally {
      policy.inFlight=false;
    }
    if(path==='/conversations') {
      if(!Array.isArray(result?.items)||!Number.isSafeInteger(result.total))fail('app_chat_response_invalid');
    } else if(!result||typeof result!=='object'||result.mapping!=null&&(typeof result.mapping!=='object'||Array.isArray(result.mapping)||Object.values(result.mapping).some(n=>!n||typeof n!=='object')))fail('app_chat_response_invalid');
    policy.failures=0;policy.notBefore=0;
    return result;
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
  if(command.op==='model_catalog') {
    const before=identity();
    if(!state.api.uBn||typeof state.scope.query?.getOrFetch!=='function')fail('app_model_catalog_unavailable');
    try {await state.scope.query.getOrFetch(state.api.uBn);}
    catch(error) {
      if(['account','user','structure'].some(key=>identity()[key]!==before[key]))fail('app_account_changed');
      const code=error?.responseStatus===429?'app_model_catalog_rate_limited':error?.responseStatus===401?'app_model_catalog_auth_expired':error?.responseStatus===403?'app_model_catalog_forbidden':'app_model_catalog_request_failed';
      fail(code);
    }
    if(appScope()!==state.scope)fail('app_model_selection_changed');
    if(['account','user','structure'].some(key=>identity()[key]!==before[key]))fail('app_account_changed');
    const groups=state.scope.get(state.api.uBn)?.data?.versionOptions;
    if(!Array.isArray(groups))fail('app_model_catalog_unavailable');
    return {identity:before,options:groups.map(group=>({label:group.label,
      options:group.options?.map(option=>({slug:option.slug,lane:option.lane,modelLabel:option.actionPillShortName??option.title,isAvailable:option.isAvailable,thinkingEffort:option.thinkingEffort??null}))})),inference_submitted:false};
  }
  if(command.op==='host_status') {
    const who=identity();
    const writer=globalThis[slot];
    return {present:true,ready:true,identity:who,bridge_revision:state.revision,
      writer_state:!writer?'unclaimed':writer.owner!==command.owner?'owned_elsewhere':writer.revision!==command.bridgeRevision?'revision_conflict':'same_owner',owned_lanes_only:true,
      saved_read_failure:writer?.owner===command.owner?writer.lastReadFailure??null:null,
      saved_read_backoff:retryStatus(),
      capabilities:{saved_read:typeof state.wire.safeGet==='function',saved_input:['uCn','yRt','pzn'].every(key=>typeof state.api[key]==='function')&&!!state.api.ozn,
        managed_stream:['prepareCompletionStream','startCompletionStream'].every(key=>typeof state.client?.[key]==='function'),
        local_functions:typeof state.api.bRt==='function'&&state.scope.get(state.shared.HAt)===true},
      active_lanes:[...state.lanes.values()].filter(l=>l.pending||l.streamActive||l.approvalPending).map(l=>({lane:l.marker,transport:l.mode})),
      inference_submitted:false};
  }
  if(['chat_list','chat_read','chat_page','follow_settle'].includes(command.op)) {
    const before=identity();
    const followed=command.op==='follow_settle'?state.lanes.get(command.lane):null;
    if(command.op==='follow_settle') {
      if(!followed||followed.mode!=='mcp'||followed.serverId!==command.conversation_id||followed.round!==command.round)fail('app_lane_declaration_changed');
      const selected=appScope();if(!selected)fail('app_scope_unavailable');
      const current=identity(selected);
      if(['account','user','structure'].some(key=>before[key]!==followed.accountIdentity[key]||current[key]!==followed.accountIdentity[key]))fail('app_account_changed');
      if(selected!==state.scope)fail('app_hooks_changed');
      if(!followed.externalResponse) {
        if(followed.settledSource===command.branch)return {settled:true,reused:true};
        fail('app_lane_declaration_changed');
      }
    }
    if(command.identity&&['account','user','structure'].some(key=>command.identity[key]!==before[key]))fail('app_account_changed');
    if(typeof state.wire.safeGet!=='function')fail('app_chat_operations_unavailable');
    if(command.op==='chat_list') {
      const result=await readSavedChat('/conversations',{query:{offset:command.offset??0,limit:command.limit??50,order:'updated',is_archived:false,hide_snorlax:false}},before);
      return {items:result.items.map(c=>({id:c.id,title:c.title??null,project_id:c.gizmo_id??null,updated_at:c.update_time??null})),total:result.total};
    }
    if(typeof command.conversation_id!=='string'||!command.conversation_id||command.conversation_id.length>256)fail('app_chat_response_invalid');
    if(followed&&typeof command.text!=='string')fail('app_chat_response_invalid');
    const settlementMatches=result=>{
      const message=result?.mapping?.[command.branch]?.message,parts=message?.content?.parts;
      return message?.id===command.branch&&message.author?.role==='assistant'&&message.status==='finished_successfully'&&message.end_turn===true&&
        ['final',null,undefined].includes(message.channel)&&message.content?.content_type==='text'&&Array.isArray(parts)&&parts.every(part=>typeof part==='string')&&parts.join('')===command.text;
    };
    const result=followed&&settlementMatches(followed.savedSource)?followed.savedSource:command.op==='chat_page'
      ?await readSavedChat('/conversations/{conversation_id}/messages',{path:{conversation_id:command.conversation_id},query:{before:command.before,num_turns:100,include_has_versions:true}},before)
      :await readSavedChat('/conversation/{conversation_id}',{path:{conversation_id:command.conversation_id}},before);
    if(command.op==='chat_read')for(const lane of state.lanes.values()) {
      if(lane.externalResponse&&lane.serverId===command.conversation_id&&['account','user','structure'].every(key=>before[key]===lane.accountIdentity[key]))lane.savedSource=result;
    }
    if(followed) {
      if((result.gizmo_id??null)!==followed.project)fail('project_membership_unconfirmed');
      if(!settlementMatches(result))fail('app_chat_response_changed');
      if(typeof state.api.uCn!=='function')fail('app_chat_input_unavailable');
      state.api.uCn(state.scope,result,{mergeActiveBranch:true,preserveAsyncStatus:true});
      followed.localId=command.conversation_id;followed.selectedParent=command.branch;
      followed.terminals=['done'];followed.externalResponse=false;followed.settledSource=command.branch;
      followed.savedSource=null;
      return {settled:true};
    }
    const publicMessage=m=>{
      if(!m)return null;
      const hidden=['system','developer'].includes(m.author?.role)||m.channel==='analysis'||m.metadata?.is_visually_hidden_from_conversation===true||['thoughts','reasoning_recap','model_editable_context'].includes(m.content?.content_type);
      return {id:m.id,role:m.author?.role,name:m.author?.name??null,model:m.metadata?.model_slug??null,end_turn:m.end_turn??null,channel:m.channel??null,recipient:m.recipient??null,status:m.status??null,created_at:m.create_time??null,
        content:hidden?null:m.content,unavailable:hidden?'hidden':null};
    };
    const mapping=result.mapping?Object.fromEntries(Object.entries(result.mapping).map(([id,n])=>[id,{id,parent:n.parent??null,children:n.children??[],message:publicMessage(n.message)}])):null;
    return {conversation_id:result.conversation_id??command.conversation_id,title:result.title??null,current_node:result.current_node??null,
      mapping,messages:Array.isArray(result.messages)?result.messages.map(publicMessage):null,
      page_info:result.page_info??null,pagination:result.pagination??null,project_id:result.gizmo_id??null};
  }
  if (command.op === 'open'||command.op==='chat_adopt') {
    if(command.identity&&['account','user','structure'].some(k=>identity()[k]!==command.identity[k]))fail('app_account_changed');
    const mode=command.returnPath??state.mode;
    if(!['mcp','local_function'].includes(mode))fail('app_return_path_invalid');
    const existing=state.lanes.get(command.lane);
    if (existing) {
      if (existing.mode!==mode || JSON.stringify(existing.signature)!==JSON.stringify(command.signature) || existing.project!==(command.project??null) || command.op==='chat_adopt'&&(existing.localId!==command.conversation_id||existing.selectedParent!==command.branch)) fail('app_lane_declaration_changed');
      if(command.readSelection===true&&!existing.selection)fail('app_lane_declaration_changed');
      return existing.selection?{localId:existing.localId,selection:existing.selection}:{localId:existing.localId};
    }
    let adopted,selection,sourceHead;
    if(command.op==='chat_adopt') {
      const before=identity(),id=command.conversation_id;
      const bound=command.bound_source_anchor!=null;
      if(typeof id!=='string'||!id||id.length>256||!['uCn','pzn'].every(key=>typeof state.api[key]==='function')||!state.api.ozn)fail('app_chat_input_unavailable');
      if(bound&&(typeof command.bound_source_anchor!=='string'||command.bound_source_anchor!==command.branch))fail('app_chat_input_unavailable');
      if(['account','user','structure'].some(key=>command.identity?.[key]!==before[key]))fail('app_account_changed');
      if([...state.lanes.values()].some(l=>[l.localId,l.serverId,l.uiLocalId].includes(id)))fail('app_chat_owned_work');
      if(!bound&&state.api.pzn(state.scope.get(state.api.ozn,id)))fail('app_chat_active_unowned');
      const raw=await readSavedChat('/conversation/{conversation_id}',{path:{conversation_id:id}},before);
      if(['account','user','structure'].some(key=>identity()[key]!==before[key]))fail('app_account_changed');
      if(!raw?.mapping?.[command.branch]||raw.conversation_origin==='tpp'||raw.conversation_origin==='flora')fail('app_chat_input_unavailable');
      const current=raw.mapping[raw.current_node]?.message;
      if(bound) {
        const anchor=raw.mapping[command.branch].message;
        const completed=anchor?.author?.role==='assistant'&&anchor.status==='finished_successfully'&&anchor.end_turn===true&&['final',null,undefined].includes(anchor.channel);
        if(!command.bound_source_interrupted&&!completed)fail('app_chat_response_invalid');
        let node=raw.current_node,child;const seen=new Set();
        while(node!==command.branch) {
          if(typeof node!=='string'||seen.has(node)||!raw.mapping[node])fail('bound_chat_branch_changed');
          seen.add(node);child=node;node=raw.mapping[node].parent;
        }
        // Rust has reconciled the failed native turn. The saved branch must
        // independently confirm that a new user follows its unfinished source.
        if(command.bound_source_interrupted&&(completed||raw.mapping[child]?.message?.author?.role!=='user'))fail('app_chat_response_invalid');
      } else if(current?.status==='in_progress'||current?.status==='inProgress'||state.api.pzn(state.scope.get(state.api.ozn,id)))fail('app_chat_active_unowned');
      if([...state.lanes.values()].some(l=>[l.localId,l.serverId,l.uiLocalId].includes(id)))fail('app_chat_owned_work');
      if((command.project??null)!==(raw.gizmo_id??null))fail('project_membership_unconfirmed');
      state.api.uCn(state.scope,raw,{mergeActiveBranch:true,preserveAsyncStatus:true});
      if(command.readSelection===true) {
        if(!state.api.nSn||!state.api.uBn||!state.api.Vwn||typeof state.scope.query?.getOrFetch!=='function')fail('app_model_selection_unavailable');
        await state.scope.query.getOrFetch(state.api.nSn,id);
        await state.scope.query.getOrFetch(state.api.uBn);
        if(appScope()!==state.scope)fail('app_model_selection_changed');
        if(['account','user','structure'].some(key=>identity()[key]!==before[key]))fail('app_account_changed');
        if([...state.lanes.values()].some(l=>[l.localId,l.serverId,l.uiLocalId].includes(id)))fail('app_chat_owned_work');
        sourceHead=raw.current_node;
        const queryHead=state.scope.get(state.api.nSn,id)?.data?.current_node;
        if(queryHead!=null&&queryHead!==sourceHead)fail('app_chat_source_changed');
        const selected=state.scope.get(state.api.Vwn,id);
        if(typeof selected?.slug!=='string'||!selected.slug||selected.thinkingEffort!=null&&typeof selected.thinkingEffort!=='string')fail('app_model_selection_unavailable');
        if(!state.scope.get(state.api.uBn)?.data?.modelConfigBySlug?.[selected.slug])fail('app_model_selection_unavailable');
        selection={slug:selected.slug,thinkingEffort:selected.thinkingEffort??null};
      }
      if(!bound&&state.api.pzn(state.scope.get(state.api.ozn,id)))fail('app_chat_active_unowned');
      adopted={id,parent:command.branch,project:raw.gizmo_id??null};
    }
    if(mode==='local_function') {
      if(typeof state.api.gRt!=='function'||typeof state.api.bRt!=='function')fail('app_local_functions_unavailable');
      state.api.gRt();
      if(!state.scope.get(state.shared.HAt))fail('app_local_functions_unavailable');
    }
    const counters=command.bound_source_anchor!=null?command.source_counters:null;
    if(counters&&(Object.keys(counters).sort().join(',')!=='dispatches,hostReplyMessages,toolMessages,userMessages'||Object.values(counters).some(v=>!Number.isSafeInteger(v)||v<0)))fail('app_chat_response_invalid');
    if(command.bound_source_anchor!=null&&!counters)fail('app_chat_response_invalid');
    let budget;
    if(command.limits!=null) {
      const {maxDispatches,deadlineUnixMs}=command.limits;
      if(typeof command.nativeThread!=='string'||!Number.isSafeInteger(maxDispatches)||maxDispatches<1||deadlineUnixMs!=null&&(!Number.isSafeInteger(deadlineUnixMs)||deadlineUnixMs<1))fail('assignment_budget_invalid');
      budget=state.workBudgets.get(command.nativeThread);
      // A missing shared assignment ledger cannot be reconstructed from one
      // lane's counters; preserve the original cap rather than resetting usage.
      if(counters&&!budget)fail('assignment_app_ledger_unavailable');
      if(budget&&(budget.maxDispatches!==maxDispatches||budget.deadlineUnixMs!==deadlineUnixMs))fail('assignment_budget_changed');
      if(!budget){budget={maxDispatches,deadlineUnixMs,reserved:0};state.workBudgets.set(command.nativeThread,budget);}
    }
    // A released final lane restores app methods; retain the owned session so
    // a later lane can reattach without replacing the authenticated app.
    for (const name of ['prepareCompletionStream','startCompletionStream']) {
      if (state.client[name] !== state.saved[name] && state.client[name] !== state.hooks[name]) fail('app_hooks_changed');
      state.client[name] = state.hooks[name];
    }
    if (state.wire.streamPost !== state.saved.streamPost && state.wire.streamPost !== state.hooks.streamPost) fail('app_hooks_changed');
    state.wire.streamPost = state.hooks.streamPost;
    const localId = adopted?.id??state.api.aCn(state.scope);
    state.lanes.set(command.lane,{localId,serverId:adopted?.id,selectedParent:adopted?.parent,selection,sourceHead,mode,budget,accountIdentity:identity(),marker:command.lane,project:command.project??null,signature: command.signature,messages: new Map(),sent: new Set(),starts: new Set(),older: new Set(),epoch: 0,previousDispatches:counters?.dispatches??0,userMessages:counters?.userMessages??0,toolMessages:counters?.toolMessages??0,hostReplyMessages:counters?.hostReplyMessages??0,roundHostReplies:0,hostReplies:new Set(),terminals: []});
    return selection?{localId,selection}:{localId};
  }
  const lane = state.lanes.get(command.lane);
  if (!lane) {
    if (command.op === 'release') return {released:true};
    fail('app_lane_missing');
  }
  const snapshot = () => {
    const hostResponses = new Set();
    for (const [scope,id] of [[state.scope,lane.localId],[lane.uiScope??state.scope,lane.uiLocalId??lane.serverId]].filter(([,id])=>typeof id==='string')) for (const node of Object.values(scope.get(state.api.JRn,id) || {})) {
      const message = node.message;
      const metadata=message?.metadata,plugin=metadata?.jit_plugin_data??metadata?.elicitation_metadata?.plugin_data;
      const target=plugin?.from_client?.target_message_id??metadata?.codex_mcp_elicitation_response?.target_message_id;
      if(typeof target==='string'){if(target.length>0&&lane.hostReplies.has(target))for(const prompt of lane.messages.values())if(hostTargets(prompt).includes(target))hostResponses.add(prompt.id);continue;}
      if (message?.id && (message.author?.role === 'assistant' || lane.mode==='mcp'&&message.author?.role==='tool') && !lane.older.has(message.id)) lane.messages.set(message.id,message);
    }
    // Public messages and our own tool calls only; never export hidden reasoning.
    const messages = [...lane.messages.values()].filter(m => lane.mode==='mcp'&&m.author?.role==='tool' || m.author?.role === 'assistant' &&
      (m.recipient === 'local.' + lane.signature.name || ((m.recipient == null || m.recipient === 'all') &&
       m.content?.content_type==='text' && [undefined,null,'commentary','final'].includes(m.channel) && !m.metadata?.is_visually_hidden_from_conversation)));
    return {round:lane.round,messages,budget:lane.budget??null,hostResponses:[...hostResponses],approvalResponses:[...lane.messages.values()].filter(m=>hostTargets(m).some(t=>lane.hostReplies.has(t))).map(m=>m.id),accepted: lane.accepted,error: lane.error,terminals: lane.terminals,roundStarts: lane.roundStarts,
      sourceToolMessages:[...lane.messages.values()].filter(m=>m.author?.role==='assistant'&&m.recipient==='api_tool.call_tool'&&m.content?.content_type==='code'&&m.status==='finished_successfully'&&!m.metadata?.is_visually_hidden_from_conversation).map(m=>({id:m.id,author:{role:'assistant'},recipient:m.recipient,status:m.status,content:m.content})),
      dispatches: (lane.previousDispatches??0)+lane.starts.size,userMessages: lane.userMessages,toolMessages: lane.toolMessages,hostReplyMessages:lane.hostReplyMessages,roundHostReplies:lane.roundHostReplies,serverId: lane.serverId,
      pending:lane.pending===true,streamActive:lane.streamActive===true,
      transportObservation:lane.transportObservation??null,
      dispatchedInput:lane.dispatchedInput??null,
      savedReadBackoff:retryStatus(),
      rejectedBeforeDispatch:lane.rejectedBeforeDispatch===true,approvalPending:lane.approvalPending===true,approvalUi:lane.approvalUi??null,
      localStreamStopped: lane.pending !== true && lane.streamActive !== true,
      serverStopState:lane.serverStopState??'not_observed',
      stopped: lane.stopRequested === true && lane.pending !== true && lane.approvalPending !== true && lane.streamActive !== true && (lane.starts.size===0&&!lane.round&&lane.error==null || lane.rejectedBeforeDispatch===true&&!lane.priorStreamUnsettled || lane.completedBeforeCancellation===true || lane.serverStopConfirmed===true)};
  };
  if (command.op === 'snapshot') return snapshot();
  if (command.op === 'follow') {
    if(lane.mode!=='mcp'||lane.serverId!==command.conversation_id)fail('app_lane_declaration_changed');
    if(lane.round===command.round)return {observing:true,reused:true};
    if(lane.pending||lane.approvalPending||lane.streamActive)fail('app_lane_busy');
    if(['account','user','structure'].some(k=>identity()[k]!==lane.accountIdentity[k]))fail('app_account_changed');
    // The public saved source was admitted by Rust. Observe that existing
    // response without calling yRt/startCompletionStream or changing its model.
    lane.round=command.round;lane.epoch++;lane.messages.clear();lane.terminals=[];lane.savedSource=null;lane.dispatchedInput=null;
    lane.error=null;lane.accepted=true;lane.roundStarts=0;lane.roundHostReplies=0;
    lane.stopRequested=false;lane.serverStopPromise=null;lane.serverStopConfirmed=false;
    lane.completedBeforeCancellation=false;lane.localCancellationStarted=false;
    lane.serverStopState='not_observed';lane.externalResponse=true;lane.settledSource=null;
    return {observing:true};
  }
  if(['approve','approve_ui','approval_prepare'].includes(command.op)) {
    const check=()=>{const current=identity();if(['account','user','structure'].some(k=>current[k]!==command.identity?.[k]))fail('app_account_changed');if(lane.stopRequested)fail('app_approval_unavailable');};
    check();
    if(lane.mode!=='mcp'||lane.error||typeof lane.serverId!=='string')fail('app_approval_unavailable');
    snapshot();
    const pending=[...lane.messages.values()].filter(m=>hostTargets(m).length&&!hostTargets(m).some(t=>lane.hostReplies.has(t)));
    if(pending.length!==1)fail('app_approval_ambiguous');
    const message=pending[0],metadata=message.metadata;
    const request=(metadata.jit_plugin_data??metadata.elicitation_metadata?.plugin_data)?.from_server;
    const body=request?.body,actions=body?.actions??[];
    const once=actions.filter(a=>!['approve_for_me','allow_low_risk'].includes(a.name)&&(a.allow_once?.target_message_id||a.allow?.target_message_id));
    if(!['confirm_action','approve_tool_in_context_scope'].includes(request?.type)||body?.connector_name!==command.connector||body.codex_mcp_elicitation||body.connection_consent||once.length!==1||once[0].allow?.passkey_credential_id)fail('app_approval_manual_required');
    const scoped=actions.flatMap(a=>a.split_action_options??[]).filter(o=>o.action?.type==='allow'&&o.action.remember_answer===true&&hostTargets(message).includes(o.action.target_message_id));
    if(scoped.length>1)fail('app_approval_ambiguous');
    const choice=scoped[0],scope=choice?'conversation':'once',target=choice?.action.target_message_id??(once[0].allow??once[0].allow_once).target_message_id;
    if(lane.approvalIntents?.has(message.id))fail('app_approval_unconfirmed');
    if(command.op==='approval_prepare'){lane.approvalNavigation=null;return {scope};}
    let click;
    if(command.op==='approve_ui') {
      if(typeof state.shared.N0t!=='function'||typeof globalThis.electronBridge?.sendMessageFromView!=='function')fail('app_approval_ui_unavailable');
      if(lane.approvalNavigation!==message.id){
        await state.shared.N0t(state.scope,'/c/'+encodeURIComponent(lane.serverId));
        globalThis.electronBridge.sendMessageFromView({type:'open-current-main-window',stealFocus:true});
        lane.approvalNavigation=message.id;
      }
      const visible=e=>e.getBoundingClientRect().width>0&&e.getBoundingClientRect().height>0;
      const text=e=>(e.innerText??e.textContent??'').replace(/\s+/g,' ').trim();
      const wait=async find=>{for(let i=0;i<60;i++){check();const found=find();if(found)return found;await new Promise(r=>setTimeout(r,100));}fail('app_approval_ui_unavailable');};
      const ids=new Set(hostTargets(message));
      lane.approvalUi={stage:'card'};
      const card=(()=>{
        const nodes=[...document.querySelectorAll('[data-chatgpt-search-message-ids],[data-message-id]')].filter(e=>visible(e)&&['data-chatgpt-search-message-ids','data-message-id'].some(k=>(e.getAttribute(k)??'').split(/[^a-zA-Z0-9_-]+/).some(id=>ids.has(id))));
        const roots=nodes.filter(e=>!nodes.some(other=>other!==e&&other.contains(e)));
        lane.approvalUi.cards=roots.length;
        return roots.length===1?roots[0]:null;
      })();
      // Cold navigation can finish after a CDP evaluation deadline. Rust polls
      // this pre-intent readiness result; no approval handler is queued yet.
      if(!card)return {pending:true};
      // Navigation can give the existing Chat a new local UI identity. Bind
      // its real scope/ID from the exact offered message card, never its title.
      let fiber=card[Object.keys(card).find(k=>k.startsWith('__reactFiber'))],uiScope,uiId;
      for(let i=0;fiber&&i<400;i++,fiber=fiber.return) {
        if(!uiId&&typeof fiber.memoizedProps?.conversationId==='string')uiId=fiber.memoizedProps.conversationId;
        for(let hook=fiber.memoizedState,j=0;hook&&j<300;hook=hook.next,j++) {
          const v=hook.memoizedState?.current;
          if(!uiScope&&typeof v?.get==='function'&&typeof v?.set==='function'&&v.node&&v.chain&&v.query)uiScope=v;
        }
        if(uiId&&uiScope)break;
      }
      if(uiScope) {
        const uiIdentity=identity(uiScope),current=identity();
        if(['account','user','structure'].some(k=>uiIdentity[k]!==current[k]))fail('app_account_changed');
        if(uiScope.get(state.api.oMn)!==state.client)fail('app_approval_ui_unavailable');
        lane.uiScope=uiScope;
      }
      if(uiId)lane.uiLocalId=uiId;
      if(choice) {
        const triggers=[...card.querySelectorAll('button[aria-haspopup]')].filter(visible);
        lane.approvalUi={stage:'menu',triggers:triggers.length};
        if(triggers.length!==1)fail('app_approval_ui_unavailable');
        if(triggers[0].getAttribute('aria-expanded')!=='true') {
          // Radix opens this menu on pointerdown, not a synthetic click alone.
          if(typeof PointerEvent!=='function')fail('app_approval_ui_unavailable');
          triggers[0].dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,cancelable:true,button:0,buttons:1,pointerType:'mouse',isPrimary:true}));
          triggers[0].dispatchEvent(new PointerEvent('pointerup',{bubbles:true,button:0,buttons:0,pointerType:'mouse',isPrimary:true}));
          triggers[0].click();
        }
        click=await wait(()=>{
          const menuId=triggers[0].getAttribute('aria-controls'),menu=menuId?document.getElementById(menuId):null;
          const menus=menu?[menu]:[...document.querySelectorAll('[role=menu]')].filter(visible);
          lane.approvalUi.menus=menus.length;
          if(menus.length!==1)return null;
          const items=[...menus[0].querySelectorAll('[role=menuitem],[role=option],button')].filter(e=>visible(e)&&text(e)===choice.label);
          lane.approvalUi.options=items.length;
          return items.length===1?items[0]:null;
        });
      } else {
        const buttons=[...card.querySelectorAll('button')].filter(e=>visible(e)&&!e.hasAttribute('aria-haspopup')&&/^Allow once(?:$|\s|⏎)/.test(text(e)));
        lane.approvalUi={stage:'once',options:buttons.length};
        if(buttons.length!==1)fail('app_approval_ui_unavailable');
        click=buttons[0];
      }
    } else {
      if(typeof state.api.jLn!=='function')fail('app_approval_ui_required');
      const offered=state.api.jLn(message);
      if(!offered||!hostTargets(message).includes(target)||offered.browserAuth||offered.connectionRequired)fail('app_approval_ui_required');
      click=()=>state.api.yRt(state.scope,{onCompletion:completion(lane),conversationId:lane.localId,projectId:lane.project??undefined,isTemporaryChat:false,model:offered.model,prompt:'',
        startupSignal:lane.controller?.signal,isRequestCurrent:()=>{check();return !lane.stopRequested;},
        toolApproval:{action:'allow',targetMessageId:target,recipient:offered.recipient,...(choice?{actionData:choice.action}:{}),
          ...(offered.isApiToolElicitation?{isApiToolElicitation:true,actionData:choice?.action??{type:'allow',target_message_id:target}}:{})}});
    }
    check();
    if(hostTargets(message).some(t=>lane.hostReplies.has(t)))fail('app_approval_unconfirmed');
    (lane.approvalIntents??=new Set()).add(message.id);
    // Both routes use the app's real action/handler; no constructed host reply.
    lane.approvalPending=true;
    if(typeof click==='function')await click();else {lane.approvalUi={stage:'clicked'};click.click();return {requested:true,scope,path:'ui',messageId:message.id};}
    for(let i=0;i<20&&!lane.hostReplies.has(target);i++){check();await new Promise(r=>setTimeout(r,100));}
    check();
    if(!lane.hostReplies.has(target))fail('app_approval_unconfirmed');
    return {requested:true,scope,path:command.op==='approve_ui'?'ui':'action'};
  }
  if(command.op==='reveal') {
    const current=identity();
    if(['account','user','structure'].some(k=>current[k]!==command.identity?.[k]))fail('app_account_changed');
    if(typeof state.shared.N0t!=='function'||typeof lane.serverId!=='string'||!lane.serverId)fail('app_reveal_unavailable');
    await state.shared.N0t(state.scope,'/c/'+encodeURIComponent(lane.serverId));
    const after=identity();
    if(['account','user','structure'].some(k=>after[k]!==command.identity?.[k]))fail('app_account_changed');
    return {requested:true};
  }
  if (command.op === 'project_membership') {
    if (!lane.project || !lane.serverId || lane.pending || lane.streamActive) fail('project_membership_unsettled');
    const before=identity();
    const conversation=await state.wire.safeGet('/conversation/{conversation_id}',{parameters:{path:{conversation_id:lane.serverId}}});
    if (JSON.stringify(identity())!==JSON.stringify(before)) fail('app_account_changed');
    if (conversation.gizmo_id!==lane.project) fail('project_membership_unconfirmed');
    return {verified:true};
  }
  if (command.op === 'stop') {
    lane.savedSource=null;
    lane.stopRequested = true;
    const who=identity(),sameAccount=()=>['account','user','structure'].every(k=>identity()[k]===who[k]&&(!lane.accountIdentity||lane.accountIdentity[k]===who[k]));
    if(!sameAccount())fail('app_account_changed');
    const completed=()=>!lane.localCancellationStarted&&lane.terminals.includes('done')&&lane.pending!==true&&lane.streamActive!==true;
    if(completed())lane.completedBeforeCancellation=true;
    // Low-level eHt/JU only cancels the client transport. It does not call the
    // installed client's stopCompletion -> /stop_conversation operation.
    try {
      if((lane.starts.size>0||lane.externalResponse)&&!lane.completedBeforeCancellation&&!lane.serverStopConfirmed){
        if(typeof lane.serverId!=='string')lane.serverStopState='server_identity_unavailable';
        else if(typeof state.client.stopCompletion!=='function')lane.serverStopState='server_stop_unavailable';
        else {
          if(!lane.serverStopPromise){
            lane.serverStopState='requested';
            lane.serverStopPromise=Promise.resolve().then(()=>{
              if(!sameAccount())fail('app_account_changed');
              return state.client.stopCompletion({conversation_id:lane.serverId,exclude_async_types:[]});
            });
          }
          const bounded=async promise=>{
            let timer;
            try{return await Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('app_stop_unconfirmed')),2000);})]);}
            finally{clearTimeout(timer);}
          };
          // A lost/rejected Stop reply must not freeze later fresh observation.
          // Reuse the one request; read server state independently, without replay.
          try{await bounded(lane.serverStopPromise);}catch{lane.serverStopState='unconfirmed';}
          if(!sameAccount())fail('app_account_changed');
          const raw=await bounded(state.wire.safeGet('/conversation/{conversation_id}/stream_status',{
            parameters:{path:{conversation_id:lane.serverId}},additionalHeaders:{'ChatGPT-Account-ID':who.account},retry:'never'}));
          if(!sameAccount())fail('app_account_changed');
          const status=(raw?.data??raw)?.status;
          lane.serverStopConfirmed=['COMPLETE','FAILURE'].includes(status);
          lane.serverStopState=lane.serverStopConfirmed?'terminal_observed':status==='IS_STREAMING'?'still_streaming':'unconfirmed';
        }
      }
    } catch {lane.serverStopState='unconfirmed';}
    // A normal terminal observed before this local cancellation also proves
    // the server finished. Local abort callbacks below cannot set that proof.
    if(completed())lane.completedBeforeCancellation=true;
    lane.localCancellationStarted=true;
    lane.controller?.abort();
    await state.api.eHt(state.scope,lane.localId);
    if(lane.uiScope&&lane.uiLocalId&&lane.uiLocalId!==lane.localId)await state.api.eHt(lane.uiScope,lane.uiLocalId);
    // A late app resume can leave a stale local active flag after cancellation.
    // Clear it only after independently observed server termination; local abort
    // alone remains insufficient and cannot authorize release or replay.
    if(lane.serverStopConfirmed)lane.streamActive=false;
    return snapshot();
  }
  if (command.op === 'submit') {
    if (lane.mode==='mcp' && command.results?.length) fail('app_mcp_result_resubmit_forbidden');
    if (lane.round === command.round) return {started:true,reused:true};
    const selected=appScope();
    if(!selected||['account','user','structure'].some(k=>identity(selected)[k]!==lane.accountIdentity[k]))fail('app_account_changed');
    if (lane.pending||lane.approvalPending||lane.streamActive) fail('app_lane_busy');
    if(lane.selection) {
      if(selected!==state.scope)fail('app_model_selection_changed');
      const queryHead=state.scope.get(state.api.nSn,lane.localId)?.data?.current_node;
      if(queryHead!=null&&queryHead!==lane.sourceHead)fail('app_chat_source_changed');
      const current=state.scope.get(state.api.Vwn,lane.localId);
      if(current?.slug!==lane.selection.slug||(current?.thinkingEffort??null)!==lane.selection.thinkingEffort)fail('app_model_selection_changed');
    }
    lane.round = command.round;
    lane.externalResponse=false;
    lane.dispatchedInput=null;
    lane.preparedUsers=new Map();
    lane.savedSource=null;
    lane.priorStreamUnsettled=lane.streamActive===true;
    lane.nonUserBootstrap=command.bootstrap===true;lane.rejectedBeforeDispatch=false;lane.admissionCause=null;
    lane.serverStopPromise=null;lane.serverStopConfirmed=false;lane.completedBeforeCancellation=false;lane.localCancellationStarted=false;lane.serverStopState='not_observed';
    lane.epoch++; lane.messages.clear(); lane.terminals = []; lane.error = null; lane.transportObservation = null; lane.accepted = false; lane.stopRequested = false; lane.roundStarts = 0; lane.roundHostReplies=0; lane.hostReplies.clear(); lane.roundPrepared = false;
    lane.older = new Set(Object.values(state.scope.get(state.api.JRn,lane.localId) || {}).map(n => n.message?.id).filter(Boolean));
    lane.controller = new AbortController();
    const message = ({role,text,hidden,sourceFingerprint}) => {
      const value={id: role==='user'&&command.messageId?command.messageId:crypto.randomUUID(),author: {role: role === 'developer' ? 'system' : role},content: {content_type: 'text',parts: [text]},metadata: {is_visually_hidden_from_conversation: hidden || role === 'developer',termux_codex_lane:command.lane},recipient: 'all',status: 'finished_successfully',end_turn: role === 'assistant' ? true : null};
      if(role==='user')lane.preparedUsers.set(value.id,{content:JSON.stringify(value.content),fingerprint:sourceFingerprint});
      return value;
    };
    const results = command.results || [], context = (command.messages || []).map(message);
    const instructions = command.instructions || [];
    lane.resultOnly = results.length > 0 && !context.some(m => m.author.role === 'user');
    const epoch = lane.epoch;
    const options = {conversationId: lane.localId,parentMessageId:lane.selectedParent??undefined,conversationOrigin:null,tppExecutionTarget:null,requestInput:{isEverydayWorkMode:false},projectId:lane.project??undefined,model: command.model,thinkingEffort: command.thinkingEffort ?? null,isTemporaryChat: false,
      onCompletion:completion(lane),
      onServerThreadIdChange: id => { lane.serverId = id; },requireResponseAcceptance: true,startupSignal: lane.controller.signal};
    let pending;
    lane.pending = true;
    try {
      const imported = results.map(r => ({id: crypto.randomUUID(),author: {role:'tool',metadata:{},name:lane.signature.name},channel:'commentary',content:{content_type:'code',text:JSON.stringify({call_id:r.id,result:r.result,tool:lane.signature.name})},metadata:{is_visually_hidden_from_conversation:true},recipient:'all',status:'finished_successfully',end_turn:null,weight:1,create_time:null,update_time:null}));
      imported.push(...context);
      if (!imported.length) fail('app_messages_required');
      for (const item of imported) lane.older.add(item.id);
      pending = state.api.yRt(state.scope,{...options,prompt:'',prependMessages:imported.slice(0,-1),
        ...(results.length ? {daybreakContinuationMessageId:results[0].id} : {}),
        userCompletionMessages:{message:imported.at(-1),extraDeveloperInstructionMessages:instructions.map(text => message({role:'developer',text,hidden:true}))}});
    } catch(error) {
      lane.pending=false;lane.error=failureCode(lane,'app_submit_failed');throw error;
    }
    Promise.resolve(pending).then(value => {
      if (epoch !== lane.epoch) return;
      lane.accepted = !!value; lane.pending = false;
    },() => { if (epoch === lane.epoch) { lane.error = failureCode(lane,'app_submit_failed'); lane.pending = false; } });
    return {started: true};
  }
  if (command.op === 'release') {
    if (lane.pending||lane.approvalPending||lane.streamActive) fail('app_lane_busy');
    if(lane.stopRequested&&!snapshot().stopped)fail('app_stop_unconfirmed');
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
