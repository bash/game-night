# Tau's Game Night

## Web Push

### Example Payload

```json
{
	"title": "Tau's Game Night is happening on November 25 🥳",
	"body": "You're warmly invited to the next Game Night on November 25, be sure to save the date :)",
	"icon": "https://tau.garden/favicon.svg",
	"requireInteraction": true,
	"actions": [
		{ "action": "save"
		, "title": "Save to Calendar"
		, "onClick":
			{ "action": "openRelative"
  		 	, "url": "/play/event.ics"
			}
		},
		{ "action": "details"
		, "title": "Show details"
		, "onClick":
			{ "action": "openRelative"
  		 	, "url": "/play"
			}
		}
	],
	"onClick":
		{ "action": "openRelative"
  	    , "url": "/play"
	    }
}
```
