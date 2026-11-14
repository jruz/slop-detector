defmodule MyAppWeb.PageLive do
  use MyAppWeb, :live_view

  def render(assigns) do
    ~H"""
    <div style="max-width: 700px">
      <p style="color: red; font-size: 14px">Hello</p>
      <span class="valid">No inline style here</span>
    </div>
    """
  end
end
