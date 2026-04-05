defmodule NestedCallArgs do
  def render_result(command) do
    wrap(run_bash(command))
  end

  defp wrap(value), do: value

  defp run_bash(command), do: command
end
