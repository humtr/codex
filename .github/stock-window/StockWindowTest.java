package com.termux.app;

import android.app.Instrumentation;
import android.content.Intent;
import android.os.SystemClock;
import android.system.Os;
import android.system.StructStat;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicInteger;
import androidx.test.platform.app.InstrumentationRegistry;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import com.termux.shared.termux.shell.command.runner.terminal.TermuxSession;
import org.junit.Test;
import org.junit.Rule;
import org.junit.rules.TestWatcher;
import org.junit.runner.Description;
import java.io.RandomAccessFile;
import org.junit.runner.RunWith;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.concurrent.TimeUnit;
import static org.junit.Assert.*;

@RunWith(AndroidJUnit4.class)
public class StockWindowTest {
    static final String BASE="/data/data/com.termux/files/home/.stock-proof";
    static final String[] IDS={"11a0fe94-2d11-79f3-8f5b-9c1958465dc1","11a0fe94-2d11-79f3-8f5b-9c1958465dc2"};
    static final String HOME="/data/data/com.termux/files/home";
    static final String HEADLESS="11a0fe94-2d11-79f3-8f5b-9c1958465dc3";
    static final String AUTOMATIC="11a0fe94-2d11-79f3-8f5b-9c1958465dc4";
    final AtomicInteger sequence=new AtomicInteger();
    final Instrumentation instrumentation=InstrumentationRegistry.getInstrumentation();
    TermuxActivity activity;
    // The APK and all terminal contents belong to this empty disposable fixture.
    // Capture failure evidence without changing timeouts, PTYs or assertions.
    @Rule public final TestWatcher failureEvidence=new TestWatcher() {
        @Override protected void failed(Throwable failure,Description description) {
            try { captureFailure(description.getMethodName()); }
            catch(Throwable diagnosticFailure) {
                android.util.Log.e("StockWindowProof","fixture diagnostics unavailable",diagnosticFailure);
            }
        }
    };
    String tail(File file,int limit) throws Exception {
        try(RandomAccessFile input=new RandomAccessFile(file,"r")) {
            long length=input.length();int size=(int)Math.min(length,limit);
            byte[] bytes=new byte[size];input.seek(length-size);input.readFully(bytes);
            return new String(bytes,StandardCharsets.UTF_8);
        }
    }
    void captureFailure(String method) throws Exception {
        StringBuilder detail=new StringBuilder("failed="+method+"\n");
        instrumentation.runOnMainSync(()->{
            if(activity==null||activity.getTermuxService()==null){detail.append("service unavailable\n");return;}
            detail.append("sessions=").append(count()).append(" window_focused=")
                .append(activity.getWindow().getDecorView().hasWindowFocus()).append('\n');
            int seen=0;
            for(TermuxSession wrapper:activity.getTermuxService().getTermuxSessions()) {
                if(seen++>=8)break;
                com.termux.terminal.TerminalSession session=wrapper.getTerminalSession();
                detail.append("pid=").append(session.getPid()).append(" selected=")
                    .append(session==activity.getCurrentSession()).append(" running=")
                    .append(session.isRunning()).append('\n');
                if(session.getEmulator()!=null) {
                    String text=session.getEmulator().getScreen().getTranscriptText();
                    detail.append(text.substring(Math.max(0,text.length()-4096))).append('\n');
                }
            }
        });
        File[] logs=new File(BASE).listFiles((dir,name)->name.endsWith(".log"));
        if(logs!=null) {
            Arrays.sort(logs,(a,b)->a.getName().compareTo(b.getName()));
            for(int i=0;i<Math.min(logs.length,16);i++)
                detail.append("file=").append(logs[i].getName()).append('\n').append(tail(logs[i],8192)).append('\n');
        }
        Files.write(new File(BASE+"/failure.txt").toPath(),detail.toString().getBytes(StandardCharsets.UTF_8));
        android.util.Log.e("StockWindowProof","bounded fixture failure evidence saved; sessions and stage logs retained");
    }
    interface Condition { boolean get() throws Exception; }
    void until(Condition condition) throws Exception {
        long end=SystemClock.elapsedRealtime()+15000;
        while(SystemClock.elapsedRealtime()<end){if(condition.get())return;SystemClock.sleep(50);}
        fail("actual Android condition timed out");
    }
    String read(String path) throws Exception { return new String(Files.readAllBytes(new File(path).toPath()),StandardCharsets.UTF_8); }
    int count(){return activity.getTermuxService().getTermuxSessionsSize();}
    int selected(){final int[] pid={-1};instrumentation.runOnMainSync(()->{if(activity.getCurrentSession()!=null)pid[0]=activity.getCurrentSession().getPid();});return pid[0];}
    int focus(String id) throws Exception {
        ProcessBuilder builder=new ProcessBuilder(BASE+"/core","termux","__terminal-focus-v1",id);
        builder.directory(new File(HOME));
        builder.environment().put("HOME","/data/data/com.termux/files/home");
        builder.environment().put("PREFIX","/data/data/com.termux/files/usr");
        builder.environment().put("TMPDIR","/data/data/com.termux/files/usr/tmp");
        builder.environment().put("PATH","/data/data/com.termux/files/usr/bin:/system/bin");
        Process process=builder.redirectErrorStream(true).redirectOutput(new File(BASE+"/focus.log")).start();
        assertTrue("bounded Manager focus",process.waitFor(20,TimeUnit.SECONDS));return process.exitValue();
    }
    void launch(String id) {
        Intent intent=new Intent("com.termux.RUN_COMMAND").setClassName("com.termux","com.termux.app.RunCommandService");
        intent.putExtra("com.termux.RUN_COMMAND_PATH",BASE+"/core");
        intent.putExtra("com.termux.RUN_COMMAND_ARGUMENTS",new String[]{"launcher",id});
        intent.putExtra("com.termux.RUN_COMMAND_WORKDIR","/data/data/com.termux/files/home");
        intent.putExtra("com.termux.RUN_COMMAND_RUNNER","terminal-session");
        // Stock initializes the PTY only after the new session has a terminal view.
        // Keeping the current session leaves this caller at PID 0 and executes nothing.
        intent.putExtra("com.termux.RUN_COMMAND_SESSION_ACTION","0");
        instrumentation.getTargetContext().startService(intent);
    }
    static class Result {
        final int code;final String output;
        Result(int code,String output){this.code=code;this.output=output;}
    }
    Result manager(String... args) throws Exception {
        ArrayList<String> command=new ArrayList<>(Arrays.asList(BASE+"/core","termux"));
        command.addAll(Arrays.asList(args));
        ProcessBuilder builder=new ProcessBuilder(command);
        // HOME in the environment does not change Android's inherited app cwd.
        // RunCommandService requires the forwarded workdir to be writable.
        builder.directory(new File(HOME));
        builder.environment().put("HOME",HOME);
        builder.environment().put("PREFIX","/data/data/com.termux/files/usr");
        builder.environment().put("TMPDIR","/data/data/com.termux/files/usr/tmp");
        builder.environment().put("PATH","/data/data/com.termux/files/usr/bin:/system/bin");
        File output=new File(BASE+"/manager-"+sequence.incrementAndGet()+".log");
        Process process=builder.redirectErrorStream(true).redirectOutput(output).start();
        assertTrue("bounded Manager display",process.waitFor(30,TimeUnit.SECONDS));
        return new Result(process.exitValue(),read(output.getPath()));
    }
    Result display(String id, String home, boolean automatic) throws Exception {
        StructStat st=Os.stat(home);
        return manager(automatic?"__terminal-auto-show-v1":"__terminal-show-v1",id,home,
                       Long.toString(st.st_dev),Long.toString(st.st_ino));
    }
    void admitted(Result result) {
        assertTrue("display admission or original pending receipt: "+result.output,
            result.code==0 || (result.code==1&&result.output.contains("display is pending")));
    }
    String executionHome(String name) throws Exception {
        String home=BASE+"/"+name;Files.createDirectories(new File(home).toPath());Os.chmod(home,0700);return home;
    }
    int ready(String id) throws Exception {
        until(()->new File(BASE+"/"+id+".ready").isFile());
        return Integer.parseInt(read(BASE+"/"+id+".ready"));
    }
    void exactForeground(int pid,String id) throws Exception {
        until(()->selected()==pid);
        File directory=new File(HOME+"/.local/share/codex/manager/terminals");
        File[] bindings=directory.listFiles((dir,name)->name.startsWith("binding-"+pid+"-"));
        assertNotNull(bindings);assertEquals("one live foreground binding",1,bindings.length);
        JSONObject binding=new JSONObject(read(bindings[0].getPath()));
        assertEquals(pid,binding.getInt("pid"));
        assertEquals("exact native foreground UUID",id,read("/proc/"+pid+"/fd/"+binding.getInt("fd")));
        until(()->{
            final boolean[] focused={false};
            instrumentation.runOnMainSync(()->focused[0]=activity.getWindow().getDecorView().hasWindowFocus());
            return focused[0];
        });
    }
    void headlessDisplayProof() throws Exception {
        String home=executionHome("execution");String foreign=executionHome("foreign");
        Files.createFile(new File(home+"/"+HEADLESS+".saved").toPath());
        Files.createFile(new File(home+"/"+AUTOMATIC+".saved").toPath());
        assertEquals(0,manager("notify","set","--auto-display","off").code);
        int before=count();int selectedBefore=selected();
        assertEquals(0,display(HEADLESS,home,true).code);
        assertEquals("OFF starts no surface",before,count());assertEquals(selectedBefore,selected());
        admitted(display(HEADLESS,home,false));int pid=ready(HEADLESS);
        until(()->count()==before+1);exactForeground(pid,HEADLESS);
        for(int i=0;i<3;i++){
            assertEquals(0,display(HEADLESS,home,false).code);
            exactForeground(pid,HEADLESS);assertEquals(before+1,count());
        }
        assertEquals("same UUID with another home is refused",1,display(HEADLESS,foreign,false).code);
        exactForeground(pid,HEADLESS);assertEquals(before+1,count());
        assertEquals(0,manager("notify","set","--auto-display","on").code);
        final Result[] raced=new Result[2];final Throwable[] errors=new Throwable[2];
        Thread click=new Thread(()->{try{raced[0]=display(AUTOMATIC,home,false);}catch(Throwable error){errors[0]=error;}});
        Thread auto=new Thread(()->{try{raced[1]=display(AUTOMATIC,home,true);}catch(Throwable error){errors[1]=error;}});
        click.start();auto.start();click.join(35000);auto.join(35000);
        assertFalse(click.isAlive());assertFalse(auto.isAlive());assertNull(errors[0]);assertNull(errors[1]);
        admitted(raced[0]);admitted(raced[1]);int automaticPid=ready(AUTOMATIC);
        until(()->count()==before+2);exactForeground(automaticPid,AUTOMATIC);
        assertEquals(0,display(HEADLESS,home,true).code);exactForeground(pid,HEADLESS);
        assertEquals(before+2,count());assertEquals(4,read(BASE+"/workloads").trim().split("\n").length);
        android.util.Log.i("StockWindowProof", "HEADLESS_DISPLAY_PROOF owner-free-synthetic-thread real-manager real-android-activity exact-foreground-uuid repeated-click auto-off-on concurrent-single-surface foreign-home-refusal no-workload-replay");
    }
    @Test public void stockManagerCreatesOnceAndSelectsEachExistingWorkload() throws Exception {
        Intent intent=new Intent(Intent.ACTION_MAIN).setClassName("com.termux","com.termux.app.TermuxActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        activity=(TermuxActivity)instrumentation.startActivitySync(intent);
        until(()->activity.getTermuxService()!=null&&count()==1);
        int baseline=count();int[] pids=new int[2];
        for(int i=0;i<2;i++){
            launch(IDS[i]);final int row=i;
            until(()->new File(BASE+"/"+IDS[row]+".ready").isFile());
            pids[i]=Integer.parseInt(read(BASE+"/"+IDS[i]+".ready"));
            final int expected=baseline+2*(i+1);until(()->count()==expected);
        }
        for(int i:new int[]{0,1,0,0,1,1}){
            assertEquals("actual Manager return",0,focus(IDS[i]));final int pid=pids[i];until(()->selected()==pid);
            assertEquals("no new caller or work terminal",baseline+4,count());
            assertEquals("no workload replay",2,read(BASE+"/workloads").trim().split("\n").length);
        }
        android.system.Os.kill(pids[0],android.system.OsConstants.SIGTERM);
        until(()->!new File("/proc/"+pids[0]).exists());
        int before=count();assertEquals("closed binding refuses before AM constructor",1,focus(IDS[0]));assertEquals(before,count());
        assertEquals(0,focus(IDS[1]));until(()->selected()==pids[1]);
        android.util.Log.i("StockWindowProof", "STOCK_WINDOW_PROOF real-stock-apk real-manager real-am-client bare-two-launches six-existing-returns no-replay stale-refusal");
        headlessDisplayProof();
    }
}
