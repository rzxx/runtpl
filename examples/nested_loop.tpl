Project report for {{ project }}:
{{foreach team in teams}}
Team: {{ team.name }}
  Members:
    {{foreach member in team.members}}
    - {{ member }}
    {{endfor}}
{{endfor}}