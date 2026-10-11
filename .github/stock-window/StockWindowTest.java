package com.termux.app;

import android.app.Instrumentation;
import android.content.Intent;
import android.os.SystemClock;
import androidx.test.platform.app.InstrumentationRegistry;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import com.termux.shared.termux.shell.command.runner.terminal.TermuxSession;
import org.junit.Test;
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
    final Instrumentation instrumentation=InstrumentationRegistry.getInstrumentation();
    TermuxActivity activity;
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
        intent.putExtra("com.termux.RUN_COMMAND_SESSION_ACTION",3);
        instrumentation.getTargetContext().startService(intent);
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
    }
}
