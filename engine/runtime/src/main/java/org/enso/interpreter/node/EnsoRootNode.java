package org.enso.interpreter.node;

import com.oracle.truffle.api.frame.FrameSlot;
import com.oracle.truffle.api.frame.FrameSlotKind;
import com.oracle.truffle.api.nodes.NodeInfo;
import com.oracle.truffle.api.nodes.RootNode;
import com.oracle.truffle.api.source.SourceSection;
import org.enso.interpreter.Language;
import org.enso.interpreter.runtime.Context;
import org.enso.interpreter.runtime.scope.LocalScope;
import org.enso.interpreter.runtime.scope.ModuleScope;

/** A common base class for all kinds of root node in Enso. */
@NodeInfo(shortName = "Root", description = "A root node for Enso computations")
public abstract class EnsoRootNode extends RootNode {
  private final String name;
  private final SourceSection sourceSection;
  private final LocalScope localScope;
  private final ModuleScope moduleScope;
  private final FrameSlot stateFrameSlot;

  /**
   * Constructs the root node.
   *
   * @param language the language instance in which this will execute
   * @param localScope a reference to the construct local scope
   * @param moduleScope a reference to the construct module scope
   * @param name the name of the construct
   * @param sourceSection a reference to the source code being executed
   */
  protected EnsoRootNode(
      Language language,
      LocalScope localScope,
      ModuleScope moduleScope,
      String name,
      SourceSection sourceSection) {
    super(language, localScope.frameDescriptor());
    this.name = name;
    this.localScope = localScope;
    this.moduleScope = moduleScope;
    this.sourceSection = sourceSection;
    this.stateFrameSlot =
        localScope.frameDescriptor().findOrAddFrameSlot("<<monadic_state>>", FrameSlotKind.Object);
  }

  /**
   * Gets a reference to the language context associated with this program.
   *
   * @return a reference to the language context
   */
  public Context getContext() {
    return Context.get(this);
  }

  /**
   * Creates a string representation of this node.
   *
   * @return a string representation of the node
   */
  @Override
  public String toString() {
    return this.name;
  }

  /**
   * Returns a language specific name of this node.
   *
   * @return a name of this node
   */
  @Override
  public String getName() {
    return this.name;
  }

  /**
   * Gets the frame slot containing the program state.
   *
   * @return the state frame slot
   */
  public FrameSlot getStateFrameSlot() {
    return stateFrameSlot;
  }

  /**
   * Gets the source code represented by this node.
   *
   * @return a reference to the source code
   */
  @Override
  public SourceSection getSourceSection() {
    return sourceSection;
  }

  /**
   * Gets the local scope this node expects to work with
   *
   * @return the local scope for this node
   */
  public LocalScope getLocalScope() {
    return localScope;
  }

  /**
   * Gets the module scope this node was defined with
   *
   * @return the module scope for this node
   */
  public ModuleScope getModuleScope() {
    return moduleScope;
  }

  /**
   * Marks this node as instrumentable by Truffle instruments API.
   *
   * @return {@code true}
   */
  @Override
  protected boolean isInstrumentable() {
    return true;
  }

  /**
   * Marks the node as cloneable for runtime splitting purposes.
   *
   * @return always true
   */
  @Override
  public boolean isCloningAllowed() {
    return true;
  }
}
