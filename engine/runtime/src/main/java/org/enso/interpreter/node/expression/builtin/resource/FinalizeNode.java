package org.enso.interpreter.node.expression.builtin.resource;

import com.oracle.truffle.api.dsl.Specialization;
import com.oracle.truffle.api.nodes.Node;
import org.enso.interpreter.dsl.BuiltinMethod;
import org.enso.interpreter.runtime.Context;
import org.enso.interpreter.runtime.data.ManagedResource;

@BuiltinMethod(
    type = "Managed_Resource",
    name = "finalize",
    description = "Finalizes a managed resource, even if it is still reachable.")
public abstract class FinalizeNode extends Node {

  static FinalizeNode build() {
    return FinalizeNodeGen.create();
  }

  abstract Object execute(Object _this);

  @Specialization
  Object doClose(ManagedResource _this) {
    Context context = Context.get(this);
    context.getResourceManager().close(_this);
    return context.getBuiltins().nothing().newInstance();
  }
}
