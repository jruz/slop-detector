defmodule MyAppWeb.PageLive do
  use MyAppWeb, :live_view

  def render(assigns) do
    ~H"""
    <div class="container">
      <p class="text-red text-sm">Hello</p>
      <span class="valid">No inline style here</span>
    </div>
    """
  end
end
