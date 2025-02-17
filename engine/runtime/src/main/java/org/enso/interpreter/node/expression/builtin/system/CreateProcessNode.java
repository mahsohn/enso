package org.enso.interpreter.node.expression.builtin.system;

import com.oracle.truffle.api.CompilerDirectives;
import com.oracle.truffle.api.dsl.Cached;
import com.oracle.truffle.api.dsl.Specialization;
import com.oracle.truffle.api.io.TruffleProcessBuilder;
import com.oracle.truffle.api.nodes.Node;
import org.enso.interpreter.dsl.BuiltinMethod;
import org.enso.interpreter.node.expression.builtin.text.util.ExpectStringNode;
import org.enso.interpreter.runtime.Context;
import org.enso.interpreter.runtime.data.Array;
import org.enso.interpreter.runtime.data.text.Text;
import org.enso.interpreter.runtime.error.PanicException;

import java.io.*;

@BuiltinMethod(
    type = "System",
    name = "create_process",
    description = "Create a system process, returning the exit code.")
public abstract class CreateProcessNode extends Node {

  static CreateProcessNode build() {
    return CreateProcessNodeGen.create();
  }

  abstract Object execute(
      Object _this,
      Object command,
      Array arguments,
      Object input,
      boolean redirect_in,
      boolean redirect_out,
      boolean redirect_err);

  @Specialization
  @CompilerDirectives.TruffleBoundary
  Object doCreate(
      Object _this,
      Object command,
      Array arguments,
      Object input,
      boolean redirectIn,
      boolean redirectOut,
      boolean redirectErr,
      @Cached ExpectStringNode expectStringNode) {
    Context ctx = Context.get(this);
    String[] cmd = new String[arguments.getItems().length + 1];
    cmd[0] = expectStringNode.execute(command);
    for (int i = 1; i <= arguments.getItems().length; i++) {
      cmd[i] = expectStringNode.execute(arguments.getItems()[i - 1]);
    }
    TruffleProcessBuilder pb = ctx.getEnvironment().newProcessBuilder(cmd);

    try {
      Process p = pb.start();
      ByteArrayInputStream in =
          new ByteArrayInputStream(expectStringNode.execute(input).getBytes());
      ByteArrayOutputStream out = new ByteArrayOutputStream();
      ByteArrayOutputStream err = new ByteArrayOutputStream();

      try (OutputStream processIn = p.getOutputStream()) {
        InputStream stdin;
        if (redirectIn) {
          stdin = ctx.getIn();
        } else {
          stdin = in;
        }
        int nread;
        byte[] buf = new byte[8096];
        try {
          while (stdin.available() > 0 && (nread = stdin.read(buf)) != -1) {
            processIn.write(buf, 0, nread);
          }
        } catch (IOException e) {
          throw new PanicException(e.getMessage(), this);
        }
      } catch (IOException ignored) {
        // Getting the output stream of a finished process results in an IOException.
        // We can ignore it at this point.
      }

      p.waitFor();

      try (InputStream processOut = p.getInputStream()) {
        OutputStream stdout;
        if (redirectOut) {
          stdout = ctx.getOut();
        } else {
          stdout = out;
        }
        int nread;
        byte[] buf = new byte[8096];
        while ((nread = processOut.read(buf)) != -1) {
          stdout.write(buf, 0, nread);
        }
      }

      try (InputStream processErr = p.getErrorStream()) {
        OutputStream stderr;
        if (redirectErr) {
          stderr = ctx.getErr();
        } else {
          stderr = err;
        }
        int nread;
        byte[] buf = new byte[8096];
        while ((nread = processErr.read(buf)) != -1) {
          stderr.write(buf, 0, nread);
        }
      }

      long exitCode = p.exitValue();
      Text returnOut = Text.create(out.toString());
      Text returnErr = Text.create(err.toString());

      return ctx.getBuiltins().system().makeSystemResult(exitCode, returnOut, returnErr);
    } catch (IOException | InterruptedException e) {
      throw new PanicException(e.getMessage(), this);
    }
  }
}
